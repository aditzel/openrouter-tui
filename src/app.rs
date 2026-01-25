use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::api::OpenRouterClient;
use crate::error::AppError;
use crate::models::{Credits, ModelDetails, ModelSummary};
use crate::ui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Credits,
    Models,
    Details,
}

#[derive(Debug)]
pub struct AppState {
    pub view: View,
    pub credits: Option<Credits>,
    pub models_all: Vec<ModelSummary>,
    pub models_filtered: Vec<ModelSummary>,
    pub selected_model: usize,
    pub selected_model_id: Option<String>,
    pub active_model: Option<ModelSummary>,
    pub details_cache: HashMap<String, ModelDetails>,
    pub pricing_requested: HashSet<String>,
    pub active_details: Option<ModelDetails>,
    pub loading: bool,
    pub status_message: String,
    pub error: Option<String>,
    pub last_refresh: Option<Duration>,
    pub search_query: String,
    pub search_active: bool,
}

pub enum AppEvent {
    Credits(Result<Credits, AppError>),
    Models(Result<Vec<ModelSummary>, AppError>),
    Details(String, Box<Result<ModelDetails, AppError>>),
}

const DETAILS_WORKERS: usize = 4;

struct DetailsQueue {
    queue: Mutex<VecDeque<String>>,
    available: Condvar,
}

impl DetailsQueue {
    fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            available: Condvar::new(),
        }
    }

    fn push(&self, model_id: String) {
        let mut queue = self.queue.lock().unwrap();
        queue.push_back(model_id);
        self.available.notify_one();
    }

    fn pop(&self) -> String {
        let mut queue = self.queue.lock().unwrap();
        loop {
            if let Some(model_id) = queue.pop_front() {
                return model_id;
            }
            queue = self.available.wait(queue).unwrap();
        }
    }
}

pub struct App {
    state: AppState,
    client: Arc<OpenRouterClient>,
    event_tx: Sender<AppEvent>,
    event_rx: Receiver<AppEvent>,
    details_queue: Arc<DetailsQueue>,
    last_refresh_at: Option<Instant>,
}

impl App {
    pub fn new(
        client: Arc<OpenRouterClient>,
        event_tx: Sender<AppEvent>,
        event_rx: Receiver<AppEvent>,
    ) -> Self {
        let details_queue = Arc::new(DetailsQueue::new());
        for _ in 0..DETAILS_WORKERS {
            let client = Arc::clone(&client);
            let tx = event_tx.clone();
            let queue = Arc::clone(&details_queue);
            thread::spawn(move || {
                loop {
                    let model_id = queue.pop();
                    let result = client.fetch_model_details(&model_id);
                    let _ = tx.send(AppEvent::Details(model_id, Box::new(result)));
                }
            });
        }

        Self {
            state: AppState {
                view: View::Credits,
                credits: None,
                models_all: Vec::new(),
                models_filtered: Vec::new(),
                selected_model: 0,
                selected_model_id: None,
                active_model: None,
                details_cache: HashMap::new(),
                pricing_requested: HashSet::new(),
                active_details: None,
                loading: false,
                status_message: "Ready".to_string(),
                error: None,
                last_refresh: None,
                search_query: String::new(),
                search_active: false,
            },
            client,
            event_tx,
            event_rx,
            details_queue,
            last_refresh_at: None,
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<()> {
        self.refresh_all();
        let tick_rate = Duration::from_millis(200);

        loop {
            self.update_refresh_time();
            terminal.draw(|frame| ui::draw(frame, &self.state))?;

            if event::poll(tick_rate)?
                && let Event::Key(key) = event::read()?
                && self.handle_key(key)
            {
                break;
            }

            while let Ok(event) = self.event_rx.try_recv() {
                self.handle_app_event(event);
            }
        }

        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.state.view == View::Models && self.state.search_active {
            match key.code {
                KeyCode::Esc => {
                    self.clear_search();
                    return false;
                }
                KeyCode::Backspace => {
                    if self.state.search_query.pop().is_some() {
                        self.apply_filter();
                    } else {
                        self.state.search_active = false;
                    }
                    return false;
                }
                KeyCode::Char(c) => {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        self.state.search_query.push(c);
                        self.apply_filter();
                    }
                    return false;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Tab => self.next_view(),
            KeyCode::BackTab => self.prev_view(),
            KeyCode::Char('r') => self.refresh_all(),
            KeyCode::Backspace => self.view_models(),
            KeyCode::Up => self.move_selection_up(),
            KeyCode::Down => self.move_selection_down(),
            KeyCode::Enter => self.open_details(),
            KeyCode::Char('/') => self.activate_search(),
            _ => {}
        }

        false
    }

    fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Credits(result) => match result {
                Ok(credits) => {
                    self.state.credits = Some(credits);
                    self.state.status_message = "Credits refreshed".to_string();
                }
                Err(error) => {
                    self.state.error = Some(format!("Credits error: {error}"));
                }
            },
            AppEvent::Models(result) => match result {
                Ok(models) => {
                    self.state.models_all = models;
                    self.state.pricing_requested.clear();
                    self.apply_filter();
                    self.spawn_pricing_updates();
                    self.state.status_message = "Models refreshed".to_string();
                }
                Err(error) => {
                    self.state.error = Some(format!("Models error: {error}"));
                }
            },
            AppEvent::Details(model_id, result) => match *result {
                Ok(details) => {
                    self.state
                        .details_cache
                        .insert(model_id.clone(), details.clone());
                    self.apply_detail_pricing(&model_id, &details);
                    if self.current_model_id().as_deref() == Some(&model_id) {
                        self.state.active_details = Some(details);
                        self.state.active_model = self.current_model();
                    }
                    self.state.status_message = "Model details loaded".to_string();
                }
                Err(error) => {
                    self.state.error = Some(format!("Model details error: {error}"));
                }
            },
        }

        self.state.loading = false;
    }

    fn refresh_all(&mut self) {
        self.state.loading = true;
        self.state.error = None;
        self.spawn_credits();
        self.spawn_models();
        self.last_refresh_at = Some(Instant::now());
    }

    fn spawn_credits(&self) {
        let client = Arc::clone(&self.client);
        let tx = self.event_tx.clone();
        thread::spawn(move || {
            let result = client.fetch_credits();
            let _ = tx.send(AppEvent::Credits(result));
        });
    }

    fn spawn_models(&self) {
        let client = Arc::clone(&self.client);
        let tx = self.event_tx.clone();
        thread::spawn(move || {
            let result = client.fetch_models();
            let _ = tx.send(AppEvent::Models(result));
        });
    }

    fn spawn_details(&self, model_id: String) {
        self.details_queue.push(model_id);
    }

    fn open_details(&mut self) {
        self.state.view = View::Details;
        if let Some(model_id) = self.current_model_id() {
            self.state.active_model = self.current_model();
            if let Some(details) = self.state.details_cache.get(&model_id).cloned() {
                self.state.active_details = Some(details);
                self.state.loading = false;
            } else {
                self.state.loading = true;
                self.state.active_details = None;
                self.request_details(model_id);
            }
        }
    }

    fn view_models(&mut self) {
        if self.state.view == View::Details {
            self.state.view = View::Models;
        }
    }

    fn next_view(&mut self) {
        self.state.view = match self.state.view {
            View::Credits => View::Models,
            View::Models => View::Details,
            View::Details => View::Credits,
        };

        if self.state.view == View::Details {
            self.open_details();
        }
    }

    fn prev_view(&mut self) {
        self.state.view = match self.state.view {
            View::Credits => View::Details,
            View::Models => View::Credits,
            View::Details => View::Models,
        };

        if self.state.view == View::Details {
            self.open_details();
        }
    }

    fn move_selection_up(&mut self) {
        if self.state.view != View::Models || self.state.models_filtered.is_empty() {
            return;
        }

        if self.state.selected_model > 0 {
            self.state.selected_model -= 1;
            self.sync_selection();
        }
    }

    fn move_selection_down(&mut self) {
        if self.state.view != View::Models || self.state.models_filtered.is_empty() {
            return;
        }

        let max_index = self.state.models_filtered.len() - 1;
        if self.state.selected_model < max_index {
            self.state.selected_model += 1;
            self.sync_selection();
        }
    }

    fn current_model_id(&self) -> Option<String> {
        self.state
            .models_filtered
            .get(self.state.selected_model)
            .map(|model| model.id.clone())
    }

    fn current_model(&self) -> Option<ModelSummary> {
        self.state
            .models_filtered
            .get(self.state.selected_model)
            .cloned()
    }

    fn activate_search(&mut self) {
        if self.state.view == View::Models {
            self.state.search_active = true;
            self.state.status_message = "Search: type to filter, Esc to clear".to_string();
        }
    }

    fn clear_search(&mut self) {
        self.state.search_query.clear();
        self.state.search_active = false;
        self.apply_filter();
    }

    fn apply_filter(&mut self) {
        let query = self.state.search_query.trim().to_lowercase();
        if query.is_empty() {
            self.state.models_filtered = self.state.models_all.clone();
        } else {
            self.state.models_filtered = self
                .state
                .models_all
                .iter()
                .filter(|model| {
                    model.id.to_lowercase().contains(&query)
                        || model.name.to_lowercase().contains(&query)
                        || model.provider.to_lowercase().contains(&query)
                })
                .cloned()
                .collect();
        }

        if let Some(selected_id) = &self.state.selected_model_id {
            if let Some(index) = self
                .state
                .models_filtered
                .iter()
                .position(|model| &model.id == selected_id)
            {
                self.state.selected_model = index;
            } else {
                self.state.selected_model = 0;
            }
        } else {
            self.state.selected_model = 0;
        }

        self.sync_selection();
    }

    fn sync_selection(&mut self) {
        let selected = self
            .state
            .models_filtered
            .get(self.state.selected_model)
            .cloned();
        self.state.selected_model_id = selected.as_ref().map(|model| model.id.clone());
        self.state.active_model = selected;
    }

    fn update_refresh_time(&mut self) {
        if let Some(last) = self.last_refresh_at {
            self.state.last_refresh = Some(last.elapsed());
        }
    }

    fn spawn_pricing_updates(&mut self) {
        let targets = self
            .state
            .models_all
            .iter()
            .filter(|model| needs_pricing(model))
            .map(|model| model.id.clone())
            .collect::<Vec<String>>();

        for model_id in targets {
            if !self.state.pricing_requested.contains(&model_id) {
                self.request_details(model_id);
            }
        }
    }

    fn request_details(&mut self, model_id: String) {
        if self.state.pricing_requested.insert(model_id.clone()) {
            self.spawn_details(model_id);
        }
    }

    fn apply_detail_pricing(&mut self, model_id: &str, details: &ModelDetails) {
        let pricing = details.pricing.clone();
        for model in &mut self.state.models_all {
            if model.id == model_id {
                model.pricing = pricing.clone();
            }
        }
        for model in &mut self.state.models_filtered {
            if model.id == model_id {
                model.pricing = pricing.clone();
            }
        }
        if self.state.selected_model_id.as_deref() == Some(model_id) {
            self.state.active_model = self.current_model();
        }
    }
}

fn needs_pricing(model: &ModelSummary) -> bool {
    let prompt = model.pricing.prompt_per_million.unwrap_or(0.0);
    let completion = model.pricing.completion_per_million.unwrap_or(0.0);
    prompt <= 0.0 || completion <= 0.0
}
