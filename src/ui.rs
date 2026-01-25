use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState, Wrap};

use crate::app::{AppState, View};
use crate::models::{Latency, ModelDetails, ModelSummary, Throughput};

pub fn draw(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.size());

    draw_header(frame, chunks[0], state);
    draw_body(frame, chunks[1], state);
    draw_footer(frame, chunks[2], state);
}

fn draw_header(frame: &mut Frame, area: Rect, state: &AppState) {
    let view_label = match state.view {
        View::Credits => "Credits",
        View::Models => "Models",
        View::Details => "Model Details",
    };

    let mut title = vec![
        Span::styled(
            "OpenRouter TUI",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(view_label, Style::default().fg(Color::Yellow)),
    ];

    if state.loading {
        title.push(Span::raw("  "));
        title.push(Span::styled(
            "Loading...",
            Style::default().fg(Color::Magenta),
        ));
    }

    if let Some(last_refresh) = state.last_refresh {
        title.push(Span::raw("  "));
        title.push(Span::styled(
            format!("Updated {}s ago", last_refresh.as_secs()),
            Style::default().fg(Color::DarkGray),
        ));
    }

    let header = Paragraph::new(Line::from(title))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Status"),
        );

    frame.render_widget(header, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, state: &AppState) {
    let hint_line = match state.view {
        View::Credits => "Tab: Models | r: Refresh | q: Quit",
        View::Models => {
            "Tab: Details | / Search | Esc Clear | r: Refresh | Shift+Tab: Prev | q: Quit"
        }
        View::Details => "Backspace: Back | r: Refresh | Tab: Credits | q: Quit",
    };

    let status_line = if let Some(error) = &state.error {
        Line::from(Span::styled(
            error.clone(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ))
    } else {
        Line::from(Span::styled(
            state.status_message.clone(),
            Style::default().fg(Color::Green),
        ))
    };

    let text = Text::from(vec![
        Line::from(Span::styled(
            hint_line,
            Style::default().fg(Color::LightBlue),
        )),
        status_line,
    ]);

    let footer = Paragraph::new(text).alignment(Alignment::Left).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );

    frame.render_widget(footer, area);
}

fn draw_body(frame: &mut Frame, area: Rect, state: &AppState) {
    match state.view {
        View::Credits => draw_credits(frame, area, state),
        View::Models => draw_models(frame, area, state),
        View::Details => draw_details(frame, area, state),
    }
}

fn draw_credits(frame: &mut Frame, area: Rect, state: &AppState) {
    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(area);

    let credits_text = if let Some(credits) = &state.credits {
        vec![
            Line::from(Span::styled(
                format!("Remaining: {:.2}", credits.remaining()),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(format!("Total: {:.2}", credits.total_credits)),
            Line::from(format!("Used: {:.2}", credits.total_usage)),
        ]
    } else {
        vec![Line::from(Span::styled(
            "Credits unavailable (provisioning key required)",
            Style::default().fg(Color::Yellow),
        ))]
    };

    let block = Paragraph::new(Text::from(credits_text))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Account Credits"),
        );

    frame.render_widget(block, body[1]);
}

fn draw_models(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    draw_search_bar(frame, chunks[0], state);

    if state.models_filtered.is_empty() {
        let empty = Paragraph::new("No models match your search")
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Models"),
            );
        frame.render_widget(empty, chunks[1]);
        return;
    }

    let header = Row::new(vec![
        Cell::from("ID"),
        Cell::from("Name"),
        Cell::from("Provider"),
        Cell::from("Context"),
        Cell::from("Prompt $/1M"),
        Cell::from("Completion $/1M"),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let rows = state
        .models_filtered
        .iter()
        .map(model_row)
        .collect::<Vec<Row>>();

    let widths = [
        Constraint::Percentage(28),
        Constraint::Percentage(20),
        Constraint::Percentage(12),
        Constraint::Percentage(10),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Models"),
        )
        .highlight_style(Style::default().fg(Color::Black).bg(Color::LightBlue))
        .highlight_symbol(">> ");

    let mut table_state = TableState::default();
    table_state.select(Some(state.selected_model));

    frame.render_stateful_widget(table, chunks[1], &mut table_state);
}

fn draw_details(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    if let Some(summary) = &state.active_model {
        draw_details_left(frame, chunks[0], summary, state.active_details.as_ref());
        draw_details_right(frame, chunks[1], state.active_details.as_ref());
    } else {
        let empty = Paragraph::new("Select a model to load details")
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Model Details"),
            );
        frame.render_widget(empty, area);
    }
}

fn draw_details_left(
    frame: &mut Frame,
    area: Rect,
    summary: &ModelSummary,
    details: Option<&ModelDetails>,
) {
    let model_id = details
        .map(|value| value.id.as_str())
        .unwrap_or(summary.id.as_str());
    let model_id = sanitize_tui_text(model_id);
    let display_name = details
        .map(|value| value.name.as_str())
        .unwrap_or(summary.name.as_str());
    let display_name = sanitize_tui_text(display_name);
    let display_provider = details
        .map(|value| value.provider.as_str())
        .unwrap_or(summary.provider.as_str());
    let display_provider = sanitize_tui_text(display_provider);
    let provider_name = details
        .and_then(|value| value.provider_name.as_deref())
        .map(sanitize_tui_text);
    let context_length = details
        .and_then(|value| value.context_length)
        .or(summary.context_length);
    let description = details
        .and_then(|value| value.description.as_deref())
        .unwrap_or("No description provided.");
    let description = sanitize_tui_text(description);

    let mut provider_line = vec![
        Span::styled("Provider: ", Style::default().fg(Color::Yellow)),
        Span::raw(display_provider),
    ];
    if let Some(name) = provider_name {
        provider_line.push(Span::raw(format!(" ({name})")));
    }

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Model ID: ", Style::default().fg(Color::Yellow)),
            Span::raw(model_id),
        ]),
        Line::from(vec![
            Span::styled("Name: ", Style::default().fg(Color::Yellow)),
            Span::raw(display_name),
        ]),
        Line::from(provider_line),
        Line::from(vec![
            Span::styled("Context: ", Style::default().fg(Color::Yellow)),
            Span::raw(format_context(context_length)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Description",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
    ];

    lines.extend(Text::from(description).lines);
    let text = Text::from(lines);

    let paragraph = Paragraph::new(text).wrap(Wrap { trim: true }).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Overview"),
    );

    frame.render_widget(paragraph, area);
}

fn draw_details_right(frame: &mut Frame, area: Rect, details: Option<&ModelDetails>) {
    if let Some(details) = details {
        draw_details_metrics(frame, area, details);
    } else {
        let loading = Paragraph::new("Loading model details...")
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Metrics"),
            );
        frame.render_widget(loading, area);
    }
}

fn draw_details_metrics(frame: &mut Frame, area: Rect, details: &ModelDetails) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let pricing_rows = vec![
        Row::new(vec![
            Cell::from("Prompt"),
            Cell::from(format_price(details.pricing.prompt_per_million)),
        ]),
        Row::new(vec![
            Cell::from("Completion"),
            Cell::from(format_price(details.pricing.completion_per_million)),
        ]),
    ];

    let pricing_widths = [Constraint::Percentage(50), Constraint::Percentage(50)];
    let pricing_table = Table::new(pricing_rows, pricing_widths)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Pricing (per 1M tokens)"),
        )
        .column_spacing(1);

    frame.render_widget(pricing_table, chunks[0]);

    let metrics_rows = metrics_rows(&details.throughput, &details.latency);
    let metrics_widths = [Constraint::Percentage(50), Constraint::Percentage(50)];
    let metrics_table = Table::new(metrics_rows, metrics_widths)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Throughput / Latency"),
        )
        .column_spacing(1);

    frame.render_widget(metrics_table, chunks[1]);
}

fn model_row(model: &ModelSummary) -> Row<'static> {
    Row::new(vec![
        Cell::from(sanitize_tui_text(&model.id)),
        Cell::from(sanitize_tui_text(&model.name)),
        Cell::from(sanitize_tui_text(&model.provider)),
        Cell::from(format_context(model.context_length)),
        Cell::from(format_price(model.pricing.prompt_per_million)),
        Cell::from(format_price(model.pricing.completion_per_million)),
    ])
}

fn draw_search_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let cursor = if state.search_active { "_" } else { "" };
    let search_text = if state.search_query.is_empty() && !state.search_active {
        "Search: / to filter".to_string()
    } else {
        format!("Search: {}{cursor}", state.search_query)
    };

    let results_text = format!(
        "Results: {} / {}",
        state.models_filtered.len(),
        state.models_all.len()
    );

    let text = Text::from(vec![
        Line::from(Span::styled(
            search_text,
            Style::default().fg(Color::LightBlue),
        )),
        Line::from(Span::styled(
            results_text,
            Style::default().fg(Color::DarkGray),
        )),
    ]);

    let search = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Search"),
    );

    frame.render_widget(search, area);
}

fn format_context(value: Option<u32>) -> String {
    value
        .map(|length| length.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn sanitize_tui_text(value: &str) -> String {
    let mut cleaned = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\n' => cleaned.push('\n'),
            '\t' => cleaned.push(' '),
            '\r' => {}
            ch if ch.is_control() => {}
            _ => cleaned.push(ch),
        }
    }
    cleaned
}

fn format_price(value: Option<f64>) -> String {
    value
        .map(|price| format!("${:.4}", price))
        .unwrap_or_else(|| "-".to_string())
}

fn metrics_rows(throughput: &Throughput, latency: &Latency) -> Vec<Row<'static>> {
    vec![
        Row::new(vec![
            Cell::from("TPS p50"),
            Cell::from(format_metric(throughput.p50, "tok/s")),
        ]),
        Row::new(vec![
            Cell::from("TPS p90"),
            Cell::from(format_metric(throughput.p90, "tok/s")),
        ]),
        Row::new(vec![
            Cell::from("TPS p99"),
            Cell::from(format_metric(throughput.p99, "tok/s")),
        ]),
        Row::new(vec![
            Cell::from("Latency p50"),
            Cell::from(format_metric(latency.p50, "ms")),
        ]),
        Row::new(vec![
            Cell::from("Latency p90"),
            Cell::from(format_metric(latency.p90, "ms")),
        ]),
        Row::new(vec![
            Cell::from("Latency p99"),
            Cell::from(format_metric(latency.p99, "ms")),
        ]),
    ]
}

fn format_metric(value: Option<f64>, suffix: &str) -> String {
    value
        .map(|metric| format!("{metric:.0} {suffix}"))
        .unwrap_or_else(|| "-".to_string())
}
