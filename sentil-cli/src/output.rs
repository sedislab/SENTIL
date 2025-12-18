use std::io::IsTerminal;

use anstyle::{AnsiColor, Style};
use indicatif::{ProgressBar, ProgressStyle};

use crate::cli::{ColorWhen, OutputFormat};

pub struct Out {
    format: OutputFormat,
    pub quiet: bool,
    pub no_input: bool,
    color: bool,
}

impl Out {
    pub fn new(
        format: OutputFormat,
        json_alias: bool,
        color: ColorWhen,
        quiet: bool,
        no_input: bool,
    ) -> Self {
        let format = if json_alias { OutputFormat::Json } else { format };
        let text = matches!(format, OutputFormat::Text);
        Self {
            format,
            quiet,
            no_input,
            color: resolve_color(color, text),
        }
    }

    pub fn is_text(&self) -> bool {
        matches!(self.format, OutputFormat::Text)
    }

    /// Whether the format is streaming JSON lines.
    pub fn is_ndjson(&self) -> bool {
        matches!(self.format, OutputFormat::Ndjson)
    }

    pub fn heading(&self, title: &str) {
        println!("{}", self.paint(title, heading()));
    }

    pub fn field(&self, label: &str, value: &str) {
        println!("  {} {value}", self.paint(&format!("{label:<11}"), dim()));
    }

    pub fn note(&self, text: &str) {
        println!("{}", self.paint(&format!("  {text}"), dim()));
    }

    pub fn paint(&self, text: &str, style: Style) -> String {
        if self.color {
            format!("{}{text}{}", style.render(), style.render_reset())
        } else {
            text.to_string()
        }
    }

    /// A spinner on stderr
    pub fn spinner(&self, message: &str) -> Option<ProgressBar> {
        if self.quiet || !self.is_text() || !std::io::stderr().is_terminal() {
            return None;
        }
        let bar = ProgressBar::new_spinner();
        if let Ok(style) = ProgressStyle::with_template("{spinner} {msg}") {
            bar.set_style(style);
        }
        bar.set_message(message.to_string());
        Some(bar)
    }
}

pub fn clear_spinner(spinner: Option<ProgressBar>) {
    if let Some(bar) = spinner {
        bar.finish_and_clear();
    }
}

fn resolve_color(when: ColorWhen, text_mode: bool) -> bool {
    if !text_mode {
        return false;
    }
    match when {
        ColorWhen::Never => false,
        ColorWhen::Always => true,
        ColorWhen::Auto => {
            if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
                return false;
            }
            if std::env::var_os("CLICOLOR_FORCE").is_some_and(|v| v != "0") {
                return true;
            }
            std::io::stdout().is_terminal()
        }
    }
}

pub fn heading() -> Style {
    Style::new().bold().fg_color(Some(AnsiColor::Cyan.into()))
}

pub fn good() -> Style {
    Style::new().fg_color(Some(AnsiColor::Green.into()))
}

pub fn bad() -> Style {
    Style::new().fg_color(Some(AnsiColor::Red.into()))
}

pub fn dim() -> Style {
    Style::new().dimmed()
}