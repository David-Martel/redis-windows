//! Windows Event Log integration as a tracing layer.
//!
//! On Windows, writes log entries to the Application Event Log under the
//! source "Redis Service". On non-Windows platforms, this is a no-op.

use tracing::Subscriber;
use tracing_subscriber::Layer;

use super::error::ServiceError;

/// The Windows Event Log source name.
pub const EVENT_SOURCE: &str = "Redis Service";

/// A tracing layer that writes events to the Windows Application Event Log.
///
/// Falls back to no-op on non-Windows platforms.
pub struct EventLogLayer {
    #[cfg(windows)]
    _source_handle: Option<isize>,
}

impl EventLogLayer {
    /// Create a new Event Log layer, registering the event source.
    ///
    /// Returns `ServiceError::EventLog` if registration fails on Windows.
    pub fn try_new() -> Result<Self, ServiceError> {
        #[cfg(windows)]
        {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            use windows::core::PCWSTR;
            use windows::Win32::System::EventLog::RegisterEventSourceW;

            let source_wide: Vec<u16> = OsStr::new(EVENT_SOURCE)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let handle = unsafe {
                RegisterEventSourceW(PCWSTR::null(), PCWSTR(source_wide.as_ptr()))
            }
            .map_err(|e| {
                ServiceError::EventLog(format!(
                    "Failed to register event source '{}': {}",
                    EVENT_SOURCE, e
                ))
            })?;

            Ok(Self {
                _source_handle: Some(handle.0 as isize),
            })
        }
        #[cfg(not(windows))]
        {
            Ok(Self {})
        }
    }

    /// Report an event to the Windows Event Log.
    #[cfg(windows)]
    fn report_event(&self, level: &tracing::Level, message: &str) {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::System::EventLog::{
            ReportEventW, EVENTLOG_ERROR_TYPE, EVENTLOG_INFORMATION_TYPE, EVENTLOG_WARNING_TYPE,
        };

        let Some(handle_val) = self._source_handle else {
            return;
        };

        let event_type = match *level {
            tracing::Level::ERROR => EVENTLOG_ERROR_TYPE,
            tracing::Level::WARN => EVENTLOG_WARNING_TYPE,
            _ => EVENTLOG_INFORMATION_TYPE,
        };

        let msg_wide: Vec<u16> = OsStr::new(message)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let msg_pcwstr = PCWSTR(msg_wide.as_ptr());
        let handle = HANDLE(handle_val as *mut core::ffi::c_void);

        unsafe {
            let _ = ReportEventW(
                handle,
                event_type,
                0,                     // category
                0,                     // event ID
                None,                  // user SID
                0,                     // data size
                Some(&[msg_pcwstr]),   // strings
                None,                  // raw data
            );
        }
    }
}

impl<S: Subscriber> Layer<S> for EventLogLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Format the event message
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let message = visitor.message;

        #[cfg(windows)]
        {
            self.report_event(event.metadata().level(), &message);
        }

        #[cfg(not(windows))]
        {
            let _ = &message;
        }
    }
}

/// Visitor that extracts the message field from a tracing event.
#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        } else if self.message.is_empty() {
            self.message = format!("{}: {:?}", field.name(), value);
        } else {
            self.message
                .push_str(&format!(", {}: {:?}", field.name(), value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else if self.message.is_empty() {
            self.message = format!("{}: {}", field.name(), value);
        } else {
            self.message
                .push_str(&format!(", {}: {}", field.name(), value));
        }
    }
}
