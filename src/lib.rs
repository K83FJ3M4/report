#![doc = include_str!("../README.md")]

use std::fmt::Arguments;
use std::cell::Cell;
use std::mem::take;
use console::{pad_str, Alignment, Style, Term};
use std::result::Result as StdResult;
use std::error::Error as StdError;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
pub use report_macros::{report, log};

thread_local! {
    static ACTIONS: Cell<Vec<Action>> = Cell::default();
    static ACTIVE: Cell<bool> = Cell::default();
}

///Custom result type without error information
/// 
///The error context is stored in thread local storage and is
///therefore not part of the result type. It is possible to use the standard
///'Result' instead of this type, in which case one would have to invoke [`error`](macro@error) manually.
///Usually, the [`Error`] type calls the macro in its 'From' implementations.
pub type Result<T = ()> = StdResult<T, Error>;

///Custom error type without context information
/// 
///This type calls the [`error`](macro@error) macro in its `From` implementation.
///The error context is then stored in thread local storage and will be printed
///by the `Drop` implementation of the [`Report`] type.
pub struct Error;

///Group of logging events
/// 
///**This type should not be used directly, but through the macros [`report`](macro@report) and [`log`](macro@log)**
pub struct Report<T: Fn() -> String> {
    message: T,
    actions: Vec<Action>,
    active: bool,
    log: bool
}

enum Action {
    Report {
        message: String,
        actions: Vec<Action>
    },
    Info(String),
    Warn(String),
    Error(String),
}

impl Report<fn() -> String> {

    ///Logs a message with the `info` prefix
    ///
    ///# Example
    ///```
    ///use report::Report;
    ///
    ///let data = 42;
    ///Report::info(format_args!("Data: {data}"));
    ///```
    pub fn info(message: Arguments) {
        if !ACTIVE.get() {
            #[cfg(feature = "color")]
            return println!("{}: {message}", Style::new().blue().apply_to("info"));
            #[cfg(not(feature = "color"))]
            return println!("info: {message}");
        }
        let mut actions = ACTIONS.take();
        actions.push(Action::Info(message.to_string()));
        ACTIONS.set(actions);
    }

    ///Logs a message with the `warning` prefix
    ///
    ///# Example
    ///```
    ///use report::Report;
    ///
    ///let data = 42;
    ///Report::warn(format_args!("Warning: {data}"));
    ///```
    pub fn warn(message: Arguments) {
        if !ACTIVE.get() {
            #[cfg(feature = "color")]
            return println!("{}: {message}", Style::new().yellow().apply_to("warning"));
            #[cfg(not(feature = "color"))]
            return println!("warning: {message}");
        }
        let mut actions = ACTIONS.take();
        actions.push(Action::Warn(message.to_string()));
        ACTIONS.set(actions);
    }

    ///Logs a message with the `error` prefix
    ///
    ///# Example
    ///```
    ///use report::Report;
    ///
    ///let data = 42;
    ///Report::error(format_args!("Error: {data}"));
    ///```
    pub fn error(message: Arguments) {
        if !ACTIVE.get() {
            #[cfg(feature = "color")]
            return println!("{}: {message}", Style::new().red().apply_to("error"));
            #[cfg(not(feature = "color"))]
            return println!("error: {message}");
        }
        let mut actions = ACTIONS.take();
        actions.push(Action::Error(message.to_string()));
        ACTIONS.set(actions);
    }

    fn print(message: String, actions: Vec<Action>) {
        let mut prefix = String::from(" ");
        let width = Term::stdout()
            .size_checked()
            .map(|(_, width)| width as usize)
            .map(|width| width.saturating_sub(4))
            .filter(|_| cfg!(feature = "frame"));

        Action::open_frame(width);
        Action::add_frame(width, format!(" {message}"));

        if !actions.is_empty() {
            Action::seperator(width);
            let max = actions.len().saturating_sub(1);
            for (index, action) in actions.into_iter().enumerate() {
                action.print(&mut prefix, width, index == max)
            }
        }

        Action::close_frame(width);
    }
}

impl Action {
    fn print(self, prefix: &mut String, width: Option<usize>, last: bool) {
        let connection = Action::get_connection(last);
        match self {
            #[cfg(not(feature = "color"))] Action::Info(message)
                => Action::add_frame(width, format!("{prefix}{connection}info: {message}")),
            #[cfg(not(feature = "color"))] Action::Warn(message)
                => Action::add_frame(width, format!("{prefix}{connection}warning: {message}")),
            #[cfg(not(feature = "color"))] Action::Error(message)
                => Action::add_frame(width, format!("{prefix}{connection}error: {message}")),
            #[cfg(feature = "color")] Action::Info(message)
                => Action::add_frame(width, format!("{prefix}{connection}{}: {message}", Style::new().blue().apply_to("info"))),
            #[cfg(feature = "color")] Action::Warn(message)
                => Action::add_frame(width, format!("{prefix}{connection}{}: {message}", Style::new().yellow().apply_to("warning"))),
            #[cfg(feature = "color")] Action::Error(message)
                => Action::add_frame(width, format!("{prefix}{connection}{}: {message}", Style::new().red().apply_to("error"))),
            Action::Report { message, actions } => {
                Action::add_frame(width, format!("{prefix}{connection}{message}"));
                prefix.push_str(Action::get_indent(last));
                let max = actions.len().saturating_sub(1);
                for (index, action) in actions.into_iter().enumerate() {
                    action.print(prefix, width, index == max)
                }
                prefix.char_indices()
                    .rev()
                    .nth(3)
                    .map(|(index, _)| prefix.truncate(index));
            }
        }
    }

    fn open_frame(width: Option<usize>) {
        let Some(width) = width else { return };
        #[cfg(feature = "unicode")]
        println!("╭{}╮", "─".repeat(width));
        #[cfg(not(feature = "unicode"))]
        println!("+{}+", "-".repeat(width));
    }
    
    fn close_frame(width: Option<usize>) {
        let Some(width) = width else { return };
        #[cfg(feature = "unicode")]
        println!("╰{}╯", "─".repeat(width));
        #[cfg(not(feature = "unicode"))]
        println!("+{}+", "-".repeat(width));
    }
    
    fn seperator(width: Option<usize>) {
        let Some(width) = width else { return };
        #[cfg(feature = "unicode")]
        println!("├─┬{}┤", "─".repeat(width.saturating_sub(2)));
        #[cfg(not(feature = "unicode"))]
        println!("+{}+", "-".repeat(width));
    }
    
    fn add_frame(width: Option<usize>, data: String) {
        let Some(width) = width else { return println!("{data}") };
        #[cfg(feature = "unicode")]
        let vertical = "│";
        #[cfg(not(feature = "unicode"))]
        let vertical = "|";
        let padded = pad_str(data.as_str(), width, Alignment::Left, Some("..."));
        println!("{vertical}{padded}{vertical}");
    }
    
    fn get_connection(last: bool) -> &'static str {
        #[cfg(feature = "unicode")]
        if last { "╰── " } else { "├── " }
        #[cfg(not(feature = "unicode"))]
        if last { "\\-- " } else { "|-- " }
    }
    
    fn get_indent(last: bool) -> &'static str {
        #[cfg(feature = "unicode")]
        if last { "    " } else { "│   " }
        #[cfg(not(feature = "unicode"))]
        if last { "    " } else { "|   " }
    }
}

impl<T: Fn() -> String> Report<T> {

    ///Collects all nested logging events and prints them
    ///
    ///When this report is dropped, it will be printed to stdout.
    ///
    ///# Example
    ///```
    ///use report::{Report, info};
    /// 
    ///let report = Report::log(|| format!("Running task"));
    ///info!("Complementary information");
    ///drop(report);
    ///```
    pub fn log(message: T) -> Self {
        Self {
            actions: ACTIONS.take(),
            message,
            active: ACTIVE.replace(true),
            log: true
        }
    }

    ///Collects all nested logging events and appends them to the
    ///preceding report
    /// 
    ///When this report is dropped and there are events available,
    ///its message will be formatted, and the events will be tagged with it.
    /// 
    ///# Example
    ///```
    ///use report::{Report, info};
    ///
    ///let report = Report::rec(|| format!("Running task"));
    ///info!("Complementary information");
    ///drop(report);
    ///```
    pub fn rec(message: T) -> Self {
        Self {
            actions: ACTIONS.take(),
            message,
            active: ACTIVE.get(),
            log: false
        }
    }
}

impl<T: Fn() -> String> Drop for Report<T> {
    fn drop(&mut self) {
        let actions = ACTIONS.take();

        if self.log {
            Report::print((self.message)(), actions)
        } else if !actions.is_empty() {
            self.actions.push(Action::Report {
                message: (self.message)(),
                actions
            })
        }

        ACTIVE.set(self.active);
        ACTIONS.set(take(&mut self.actions));
    }
}

///Default implementation, which does not provide any additional information
impl Debug for Error {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter.write_str("Error")
    }
}

///Default implementation, which does not provide any additional information
impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter.write_str("Error")
    }
}

///Constructs a new `Error` and moves the contex to thread local storage
///by calling the [`error`](macro@error) macro.
impl<T: StdError> From<T> for Error {
    fn from(error: T) -> Self {
        Report::error(format_args!("{error}"));
        Error
    }
}

///Logs a message with the `info` prefix
///
 ///# Example
///```
///use report::info;
///
///let data = 42;
///info!("Data: {data}");
///```
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        report::Report::info(format_args!($($arg)*))
    };
}

///Logs a message with the `warning` prefix
///
 ///# Example
///```
///use report::warn;
///
///let data = 42;
///warn!("Warning: {data}");
///```
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        report::Report::warn(format_args!($($arg)*))
    };
}

///Logs a message with the `error` prefix
///
 ///# Example
///```
///use report::error;
///
///let data = 42;
///error!("Error: {data}");
///```
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        report::Report::error(format_args!($($arg)*))
    };
}

///Log error message and return from function
///
///This macro expands to the following code:
///```
///return Err({
///    report::Report::error(format_args!(args));
///    report::Error
///})
///```
/// 
///# Example
///```
///use report::{bail, Result};
///
///fn function() -> Result {
///    bail!("Error message")
///}
///```
#[macro_export]
macro_rules! bail {
    ($($arg:tt)*) => {
        return Err({
            report::Report::error(format_args!($($arg)*));
            report::Error
        })
    };
}
