use serde::{Deserialize, Serialize};
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::{collections::HashSet, collections::VecDeque};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::source_selection::SourceSelection;

const MAX_EXPLORER_PATHS: usize = 100;
const COPY_AGGREGATION_WINDOW: Duration = Duration::from_millis(250);
pub const EXPLORER_REQUEST_EVENT: &str = "explorer://request";
pub const EXPLORER_ERROR_EVENT: &str = "explorer://error";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExplorerAction {
    SetSource,
    SetDestination,
    Copy,
    Paste,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExplorerRequest {
    pub id: String,
    pub action: ExplorerAction,
    pub paths: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_selection: Option<SourceSelection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExplorerErrorPayload {
    pub id: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExplorerIntegrationStatus {
    pub supported: bool,
    pub installed: bool,
    pub healthy: bool,
    pub executable_path: String,
    pub expected_commands: usize,
    pub matching_commands: usize,
    pub problems: Vec<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExplorerIntegrationError {
    pub code: String,
    pub message: String,
}

impl ExplorerIntegrationError {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy)]
struct ExplorerVerb {
    key: &'static str,
    label: &'static str,
    action: ExplorerAction,
    single_selection: bool,
}

const EXPLORER_VERBS: [ExplorerVerb; 9] = [
    ExplorerVerb {
        key: r"Software\Classes\*\shell\OffloadKit.SetSource",
        label: "Đặt làm Source trong OffloadKit",
        action: ExplorerAction::SetSource,
        single_selection: false,
    },
    ExplorerVerb {
        key: r"Software\Classes\*\shell\OffloadKit.Copy",
        label: "Copy bằng OffloadKit",
        action: ExplorerAction::Copy,
        single_selection: false,
    },
    ExplorerVerb {
        key: r"Software\Classes\Directory\shell\OffloadKit.SetSource",
        label: "Đặt làm Source trong OffloadKit",
        action: ExplorerAction::SetSource,
        single_selection: false,
    },
    ExplorerVerb {
        key: r"Software\Classes\Directory\shell\OffloadKit.SetDestination",
        label: "Đặt làm Destination trong OffloadKit",
        action: ExplorerAction::SetDestination,
        single_selection: true,
    },
    ExplorerVerb {
        key: r"Software\Classes\Directory\shell\OffloadKit.Copy",
        label: "Copy bằng OffloadKit",
        action: ExplorerAction::Copy,
        single_selection: false,
    },
    ExplorerVerb {
        key: r"Software\Classes\Directory\shell\OffloadKit.Paste",
        label: "Paste và bắt đầu transfer bằng OffloadKit",
        action: ExplorerAction::Paste,
        single_selection: true,
    },
    ExplorerVerb {
        key: r"Software\Classes\Directory\Background\shell\OffloadKit.SetSource",
        label: "Đặt thư mục hiện tại làm Source trong OffloadKit",
        action: ExplorerAction::SetSource,
        single_selection: false,
    },
    ExplorerVerb {
        key: r"Software\Classes\Directory\Background\shell\OffloadKit.SetDestination",
        label: "Đặt thư mục hiện tại làm Destination trong OffloadKit",
        action: ExplorerAction::SetDestination,
        single_selection: true,
    },
    ExplorerVerb {
        key: r"Software\Classes\Directory\Background\shell\OffloadKit.Paste",
        label: "Paste và bắt đầu transfer bằng OffloadKit",
        action: ExplorerAction::Paste,
        single_selection: true,
    },
];

fn explorer_verbs() -> &'static [ExplorerVerb] {
    &EXPLORER_VERBS
}

#[derive(Clone)]
struct RegistryScope {
    base: String,
}

impl RegistryScope {
    fn new(base: impl Into<String>) -> Self {
        Self { base: base.into() }
    }

    fn production() -> Self {
        Self::new("")
    }

    fn qualify(&self, relative: &str) -> String {
        if self.base.is_empty() {
            relative.to_owned()
        } else {
            format!("{}\\{relative}", self.base.trim_end_matches('\\'))
        }
    }
}

#[derive(Clone, Debug)]
enum RegistryAccessError {
    NotFound,
    Other(String),
}

impl fmt::Display for RegistryAccessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("registry key or value was not found"),
            Self::Other(message) => formatter.write_str(message),
        }
    }
}

trait RegistryAccess {
    fn key_exists(&self, key: &str) -> Result<bool, RegistryAccessError>;
    fn create_key(&self, key: &str) -> Result<(), RegistryAccessError>;
    fn set_string(
        &self,
        key: &str,
        value_name: Option<&str>,
        value: &str,
    ) -> Result<(), RegistryAccessError>;
    fn get_string(
        &self,
        key: &str,
        value_name: Option<&str>,
    ) -> Result<String, RegistryAccessError>;
    fn delete_tree(&self, key: &str) -> Result<(), RegistryAccessError>;
}

fn build_registry_command(
    executable: &Path,
    action: &ExplorerAction,
) -> Result<String, ExplorerIntegrationError> {
    let executable = executable.to_str().ok_or_else(|| {
        ExplorerIntegrationError::new(
            "invalidExecutablePath",
            "OffloadKit executable path is not valid Unicode",
        )
    })?;
    let action = match action {
        ExplorerAction::SetSource => "set-source",
        ExplorerAction::SetDestination => "set-destination",
        ExplorerAction::Copy => "copy",
        ExplorerAction::Paste => "paste",
    };
    Ok(format!(
        "{} --explorer-action {action} --path \"%V\"",
        quote_windows_argument(executable)
    ))
}

fn quote_windows_argument(value: &str) -> String {
    let mut quoted = String::from("\"");
    let mut backslashes = 0usize;
    for character in value.chars() {
        match character {
            '\\' => backslashes += 1,
            '"' => {
                quoted.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                quoted.push('"');
                backslashes = 0;
            }
            _ => {
                quoted.extend(std::iter::repeat_n('\\', backslashes));
                quoted.push(character);
                backslashes = 0;
            }
        }
    }
    quoted.extend(std::iter::repeat_n('\\', backslashes * 2));
    quoted.push('"');
    quoted
}

fn integration_status_with_registry<R: RegistryAccess>(
    registry: &R,
    scope: &RegistryScope,
    executable: &Path,
) -> Result<ExplorerIntegrationStatus, ExplorerIntegrationError> {
    let executable_text = executable
        .to_str()
        .ok_or_else(|| {
            ExplorerIntegrationError::new(
                "invalidExecutablePath",
                "OffloadKit executable path is not valid Unicode",
            )
        })?
        .to_owned();
    let mut matching_commands = 0;
    let mut problems = Vec::new();

    for verb in explorer_verbs() {
        let expected = build_registry_command(executable, &verb.action)?;
        let command_key = scope.qualify(&format!("{}\\command", verb.key));
        match registry.get_string(&command_key, None) {
            Ok(actual) if actual == expected => matching_commands += 1,
            Ok(_) => problems.push(format!("Registry command does not match: {}", verb.key)),
            Err(RegistryAccessError::NotFound) => {
                problems.push(format!("Registry command is missing: {}", verb.key));
            }
            Err(error) => {
                return Err(ExplorerIntegrationError::new(
                    "registryReadFailed",
                    format!("Cannot read {}: {error}", verb.key),
                ));
            }
        }
    }

    let healthy = matching_commands == explorer_verbs().len();
    let message = (!problems.is_empty()).then(|| problems.join("; "));
    Ok(ExplorerIntegrationStatus {
        supported: cfg!(windows),
        installed: healthy,
        healthy,
        executable_path: executable_text,
        expected_commands: explorer_verbs().len(),
        matching_commands,
        problems,
        message,
    })
}

fn install_with_registry<R: RegistryAccess>(
    registry: &R,
    scope: &RegistryScope,
    executable: &Path,
) -> Result<ExplorerIntegrationStatus, ExplorerIntegrationError> {
    let icon = executable.to_str().ok_or_else(|| {
        ExplorerIntegrationError::new(
            "invalidExecutablePath",
            "OffloadKit executable path is not valid Unicode",
        )
    })?;
    let mut created_keys = Vec::new();

    for verb in explorer_verbs() {
        let key = scope.qualify(verb.key);
        let existed = registry.key_exists(&key).map_err(|error| {
            ExplorerIntegrationError::new(
                "registryInstallFailed",
                format!("Cannot inspect {}: {error}", verb.key),
            )
        })?;
        if let Err(error) = registry.create_key(&key) {
            return Err(rollback_install(registry, &created_keys, verb.key, error));
        }
        if !existed {
            created_keys.push(key.clone());
        }

        let command = build_registry_command(executable, &verb.action)?;
        let writes = [
            registry.set_string(&key, None, verb.label),
            registry.set_string(&key, Some("Icon"), icon),
            registry.set_string(&key, Some("Position"), "Bottom"),
        ];
        for write in writes {
            if let Err(error) = write {
                return Err(rollback_install(registry, &created_keys, verb.key, error));
            }
        }
        if verb.single_selection {
            if let Err(error) = registry.set_string(&key, Some("MultiSelectModel"), "Single") {
                return Err(rollback_install(registry, &created_keys, verb.key, error));
            }
        }

        let command_key = format!("{key}\\command");
        if let Err(error) = registry.create_key(&command_key) {
            return Err(rollback_install(registry, &created_keys, verb.key, error));
        }
        if let Err(error) = registry.set_string(&command_key, None, &command) {
            return Err(rollback_install(registry, &created_keys, verb.key, error));
        }
    }

    let status = integration_status_with_registry(registry, scope, executable)?;
    if !status.healthy {
        return Err(rollback_install(
            registry,
            &created_keys,
            "read-back",
            RegistryAccessError::Other(status.problems.join("; ")),
        ));
    }
    Ok(status)
}

fn rollback_install<R: RegistryAccess>(
    registry: &R,
    created_keys: &[String],
    failed_key: &str,
    failure: RegistryAccessError,
) -> ExplorerIntegrationError {
    let mut rollback_errors = Vec::new();
    for key in created_keys.iter().rev() {
        if let Err(error) = registry.delete_tree(key) {
            rollback_errors.push(format!("{key}: {error}"));
        }
    }
    let rollback = if rollback_errors.is_empty() {
        "created keys were rolled back".to_owned()
    } else {
        format!("rollback also failed: {}", rollback_errors.join("; "))
    };
    ExplorerIntegrationError::new(
        "registryInstallFailed",
        format!("Cannot install {failed_key}: {failure}; {rollback}"),
    )
}

fn uninstall_with_registry<R: RegistryAccess>(
    registry: &R,
    scope: &RegistryScope,
    executable: &Path,
) -> Result<ExplorerIntegrationStatus, ExplorerIntegrationError> {
    for verb in explorer_verbs() {
        let key = scope.qualify(verb.key);
        registry.delete_tree(&key).map_err(|error| {
            ExplorerIntegrationError::new(
                "registryUninstallFailed",
                format!("Cannot remove {}: {error}", verb.key),
            )
        })?;
    }
    integration_status_with_registry(registry, scope, executable)
}

#[cfg(windows)]
pub fn install_explorer_integration_for_current_user(
) -> Result<ExplorerIntegrationStatus, ExplorerIntegrationError> {
    let executable = std::env::current_exe().map_err(|error| {
        ExplorerIntegrationError::new(
            "currentExecutableUnavailable",
            format!("Cannot resolve the OffloadKit executable: {error}"),
        )
    })?;
    install_with_registry(&WindowsRegistry, &RegistryScope::production(), &executable)
}

#[cfg(not(windows))]
pub fn install_explorer_integration_for_current_user(
) -> Result<ExplorerIntegrationStatus, ExplorerIntegrationError> {
    Err(ExplorerIntegrationError::new(
        "unsupportedPlatform",
        "Windows Explorer integration is only available on Windows",
    ))
}

#[cfg(windows)]
pub fn uninstall_explorer_integration_for_current_user(
) -> Result<ExplorerIntegrationStatus, ExplorerIntegrationError> {
    let executable = std::env::current_exe().map_err(|error| {
        ExplorerIntegrationError::new(
            "currentExecutableUnavailable",
            format!("Cannot resolve the OffloadKit executable: {error}"),
        )
    })?;
    uninstall_with_registry(&WindowsRegistry, &RegistryScope::production(), &executable)
}

#[cfg(not(windows))]
pub fn uninstall_explorer_integration_for_current_user(
) -> Result<ExplorerIntegrationStatus, ExplorerIntegrationError> {
    Err(ExplorerIntegrationError::new(
        "unsupportedPlatform",
        "Windows Explorer integration is only available on Windows",
    ))
}

#[cfg(windows)]
pub fn explorer_integration_status_for_current_user(
) -> Result<ExplorerIntegrationStatus, ExplorerIntegrationError> {
    let executable = std::env::current_exe().map_err(|error| {
        ExplorerIntegrationError::new(
            "currentExecutableUnavailable",
            format!("Cannot resolve the OffloadKit executable: {error}"),
        )
    })?;
    integration_status_with_registry(&WindowsRegistry, &RegistryScope::production(), &executable)
}

#[cfg(not(windows))]
pub fn explorer_integration_status_for_current_user(
) -> Result<ExplorerIntegrationStatus, ExplorerIntegrationError> {
    Ok(ExplorerIntegrationStatus {
        supported: false,
        installed: false,
        healthy: false,
        executable_path: String::new(),
        expected_commands: 0,
        matching_commands: 0,
        problems: vec!["Windows Explorer integration is only available on Windows".to_owned()],
        message: Some("Windows Explorer integration is only available on Windows".to_owned()),
    })
}

#[tauri::command]
pub fn install_explorer_integration() -> Result<ExplorerIntegrationStatus, ExplorerIntegrationError>
{
    install_explorer_integration_for_current_user()
}

#[tauri::command]
pub fn uninstall_explorer_integration(
) -> Result<ExplorerIntegrationStatus, ExplorerIntegrationError> {
    uninstall_explorer_integration_for_current_user()
}

#[tauri::command]
pub fn explorer_integration_status() -> Result<ExplorerIntegrationStatus, ExplorerIntegrationError>
{
    explorer_integration_status_for_current_user()
}

#[cfg(windows)]
struct WindowsRegistry;

#[cfg(windows)]
impl RegistryAccess for WindowsRegistry {
    fn key_exists(&self, key: &str) -> Result<bool, RegistryAccessError> {
        windows_registry::key_exists(key)
    }

    fn create_key(&self, key: &str) -> Result<(), RegistryAccessError> {
        windows_registry::create_key(key)
    }

    fn set_string(
        &self,
        key: &str,
        value_name: Option<&str>,
        value: &str,
    ) -> Result<(), RegistryAccessError> {
        windows_registry::set_string(key, value_name, value)
    }

    fn get_string(
        &self,
        key: &str,
        value_name: Option<&str>,
    ) -> Result<String, RegistryAccessError> {
        windows_registry::get_string(key, value_name)
    }

    fn delete_tree(&self, key: &str) -> Result<(), RegistryAccessError> {
        windows_registry::delete_tree(key)
    }
}

#[cfg(windows)]
mod windows_registry {
    use super::RegistryAccessError;
    use std::ffi::c_void;
    use std::ptr;

    type Hkey = *mut c_void;
    const HKEY_CURRENT_USER: Hkey = 0x80000001usize as Hkey;
    const ERROR_SUCCESS: i32 = 0;
    const ERROR_FILE_NOT_FOUND: i32 = 2;
    const KEY_QUERY_VALUE: u32 = 0x0001;
    const KEY_SET_VALUE: u32 = 0x0002;
    const KEY_CREATE_SUB_KEY: u32 = 0x0004;
    const KEY_READ: u32 = 0x20019;
    const REG_OPTION_NON_VOLATILE: u32 = 0;
    const REG_SZ: u32 = 1;

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegOpenKeyExW(
            hkey: Hkey,
            sub_key: *const u16,
            options: u32,
            desired_access: u32,
            result: *mut Hkey,
        ) -> i32;
        fn RegCreateKeyExW(
            hkey: Hkey,
            sub_key: *const u16,
            reserved: u32,
            class: *mut u16,
            options: u32,
            desired_access: u32,
            security_attributes: *const c_void,
            result: *mut Hkey,
            disposition: *mut u32,
        ) -> i32;
        fn RegSetValueExW(
            hkey: Hkey,
            value_name: *const u16,
            reserved: u32,
            value_type: u32,
            data: *const u8,
            data_size: u32,
        ) -> i32;
        fn RegQueryValueExW(
            hkey: Hkey,
            value_name: *const u16,
            reserved: *mut u32,
            value_type: *mut u32,
            data: *mut u8,
            data_size: *mut u32,
        ) -> i32;
        fn RegDeleteTreeW(hkey: Hkey, sub_key: *const u16) -> i32;
        fn RegCloseKey(hkey: Hkey) -> i32;
    }

    struct OpenKey(Hkey);

    impl Drop for OpenKey {
        fn drop(&mut self) {
            unsafe {
                RegCloseKey(self.0);
            }
        }
    }

    pub fn key_exists(key: &str) -> Result<bool, RegistryAccessError> {
        let key = wide(key);
        let mut handle = ptr::null_mut();
        let status =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, key.as_ptr(), 0, KEY_READ, &mut handle) };
        match status {
            ERROR_SUCCESS => {
                drop(OpenKey(handle));
                Ok(true)
            }
            ERROR_FILE_NOT_FOUND => Ok(false),
            _ => Err(os_error("open registry key", status)),
        }
    }

    pub fn create_key(key: &str) -> Result<(), RegistryAccessError> {
        open_or_create(key).map(drop)
    }

    pub fn set_string(
        key: &str,
        value_name: Option<&str>,
        value: &str,
    ) -> Result<(), RegistryAccessError> {
        let handle = open_or_create(key)?;
        let value_name = value_name.map(wide);
        let value_name_pointer = value_name
            .as_ref()
            .map_or(ptr::null(), |name| name.as_ptr());
        let value = wide(value);
        let status = unsafe {
            RegSetValueExW(
                handle.0,
                value_name_pointer,
                0,
                REG_SZ,
                value.as_ptr().cast(),
                (value.len() * size_of::<u16>()) as u32,
            )
        };
        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(os_error("write registry string", status))
        }
    }

    pub fn get_string(key: &str, value_name: Option<&str>) -> Result<String, RegistryAccessError> {
        let key_wide = wide(key);
        let mut raw_handle = ptr::null_mut();
        let open_status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                key_wide.as_ptr(),
                0,
                KEY_QUERY_VALUE,
                &mut raw_handle,
            )
        };
        if open_status == ERROR_FILE_NOT_FOUND {
            return Err(RegistryAccessError::NotFound);
        }
        if open_status != ERROR_SUCCESS {
            return Err(os_error("open registry key for reading", open_status));
        }
        let handle = OpenKey(raw_handle);
        let value_name = value_name.map(wide);
        let value_name_pointer = value_name
            .as_ref()
            .map_or(ptr::null(), |name| name.as_ptr());
        let mut value_type = 0u32;
        let mut byte_count = 0u32;
        let size_status = unsafe {
            RegQueryValueExW(
                handle.0,
                value_name_pointer,
                ptr::null_mut(),
                &mut value_type,
                ptr::null_mut(),
                &mut byte_count,
            )
        };
        if size_status == ERROR_FILE_NOT_FOUND {
            return Err(RegistryAccessError::NotFound);
        }
        if size_status != ERROR_SUCCESS {
            return Err(os_error("query registry string size", size_status));
        }
        if value_type != REG_SZ {
            return Err(RegistryAccessError::Other(format!(
                "registry value at {key} is not REG_SZ"
            )));
        }

        let mut buffer = vec![0u16; (byte_count as usize).div_ceil(size_of::<u16>())];
        let read_status = unsafe {
            RegQueryValueExW(
                handle.0,
                value_name_pointer,
                ptr::null_mut(),
                &mut value_type,
                buffer.as_mut_ptr().cast(),
                &mut byte_count,
            )
        };
        if read_status != ERROR_SUCCESS {
            return Err(os_error("read registry string", read_status));
        }
        if buffer.last() == Some(&0) {
            buffer.pop();
        }
        String::from_utf16(&buffer).map_err(|error| {
            RegistryAccessError::Other(format!("registry string is invalid UTF-16: {error}"))
        })
    }

    pub fn delete_tree(key: &str) -> Result<(), RegistryAccessError> {
        let key = wide(key);
        let status = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, key.as_ptr()) };
        match status {
            ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => Ok(()),
            _ => Err(os_error("delete registry tree", status)),
        }
    }

    fn open_or_create(key: &str) -> Result<OpenKey, RegistryAccessError> {
        let key = wide(key);
        let mut handle = ptr::null_mut();
        let mut disposition = 0u32;
        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                0,
                ptr::null_mut(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE | KEY_CREATE_SUB_KEY,
                ptr::null(),
                &mut handle,
                &mut disposition,
            )
        };
        if status == ERROR_SUCCESS {
            Ok(OpenKey(handle))
        } else {
            Err(os_error("create registry key", status))
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }

    fn os_error(operation: &str, status: i32) -> RegistryAccessError {
        RegistryAccessError::Other(format!(
            "{operation} failed with Windows error {status}: {}",
            std::io::Error::from_raw_os_error(status)
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerActivation {
    None,
    Request(ExplorerRequest),
    Error(ExplorerErrorPayload),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerEvent {
    Request(ExplorerRequest),
    Error(ExplorerErrorPayload),
}

#[derive(Default)]
struct ExplorerPendingInner {
    frontend_ready: bool,
    pending: VecDeque<ExplorerEvent>,
    seen_request_ids: HashSet<String>,
    in_flight_request_ids: HashSet<String>,
    consumed_request_ids: VecDeque<String>,
}

#[derive(Default)]
pub struct ExplorerPendingState {
    inner: Mutex<ExplorerPendingInner>,
}

impl ExplorerPendingState {
    pub fn new(startup_activation: ExplorerActivation) -> Self {
        let state = Self::default();
        state.enqueue(startup_activation);
        state
    }

    pub fn enqueue(&self, activation: ExplorerActivation) -> Vec<ExplorerEvent> {
        let mut inner = self.inner.lock().unwrap();
        let event = match activation {
            ExplorerActivation::None => return Vec::new(),
            ExplorerActivation::Request(request) => {
                if !inner.seen_request_ids.insert(request.id.clone()) {
                    return Vec::new();
                }
                ExplorerEvent::Request(request)
            }
            ExplorerActivation::Error(error) => ExplorerEvent::Error(error),
        };

        if inner.frontend_ready {
            if let ExplorerEvent::Request(request) = &event {
                inner.in_flight_request_ids.insert(request.id.clone());
            }
            vec![event]
        } else {
            inner.pending.push_back(event);
            Vec::new()
        }
    }

    pub fn mark_frontend_ready(&self) -> Vec<ExplorerEvent> {
        let mut inner = self.inner.lock().unwrap();
        inner.frontend_ready = true;
        let events: Vec<ExplorerEvent> = inner.pending.drain(..).collect();
        for event in &events {
            if let ExplorerEvent::Request(request) = event {
                inner.in_flight_request_ids.insert(request.id.clone());
            }
        }
        events
    }

    pub fn requeue_after_emit_failure(&self, event: ExplorerEvent) {
        let mut inner = self.inner.lock().unwrap();
        if let ExplorerEvent::Request(request) = &event {
            inner.in_flight_request_ids.remove(&request.id);
        }
        inner.pending.push_front(event);
    }

    pub fn acknowledge(&self, request_id: &str) -> bool {
        let mut inner = self.inner.lock().unwrap();
        if !inner.in_flight_request_ids.remove(request_id) {
            return false;
        }
        inner.consumed_request_ids.push_back(request_id.to_owned());
        true
    }

    #[cfg(test)]
    fn pending_request_count(&self) -> usize {
        self.inner
            .lock()
            .unwrap()
            .pending
            .iter()
            .filter(|event| matches!(event, ExplorerEvent::Request(_)))
            .count()
    }

    #[cfg(test)]
    fn in_flight_request_count(&self) -> usize {
        self.inner.lock().unwrap().in_flight_request_ids.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplorerRequestError {
    message: String,
}

impl ExplorerRequestError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ExplorerRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ExplorerRequestError {}

trait FileDropClipboard {
    fn read_paths(&self) -> Result<Vec<PathBuf>, ExplorerRequestError>;
    fn write_paths(&self, paths: &[PathBuf]) -> Result<(), ExplorerRequestError>;
}

#[derive(Default)]
struct ExplorerCopyAggregationInner {
    paths: Vec<PathBuf>,
    last_update: Option<Instant>,
}

#[derive(Default)]
pub struct ExplorerCopyAggregationState {
    inner: Mutex<ExplorerCopyAggregationInner>,
}

impl ExplorerCopyAggregationState {
    pub fn new(startup_activation: &ExplorerActivation) -> Self {
        let state = Self::default();
        if let ExplorerActivation::Request(request) = startup_activation {
            if matches!(request.action, ExplorerAction::Copy) {
                let mut inner = state.inner.lock().unwrap();
                inner.paths = request.paths.clone();
                inner.last_update = Some(Instant::now());
            }
        }
        state
    }

    pub fn merge(&self, request: ExplorerRequest) -> ExplorerRequest {
        self.merge_at(request, Instant::now())
    }

    fn merge_at(&self, mut request: ExplorerRequest, now: Instant) -> ExplorerRequest {
        if !matches!(request.action, ExplorerAction::Copy) {
            return request;
        }
        let mut inner = self.inner.lock().unwrap();
        let within_window = inner
            .last_update
            .is_some_and(|last_update| now.duration_since(last_update) <= COPY_AGGREGATION_WINDOW);
        if within_window {
            let mut combined = inner.paths.clone();
            combined.extend(request.paths);
            request.paths = combined;
        }
        inner.paths = request.paths.clone();
        inner.last_update = Some(now);
        request
    }
}

struct NativeFileDropClipboard;

impl FileDropClipboard for NativeFileDropClipboard {
    fn read_paths(&self) -> Result<Vec<PathBuf>, ExplorerRequestError> {
        native_file_drop::read_paths().map_err(|error| {
            ExplorerRequestError::new(format!("Cannot read Windows File Drop clipboard: {error}"))
        })
    }

    fn write_paths(&self, paths: &[PathBuf]) -> Result<(), ExplorerRequestError> {
        native_file_drop::write_paths(paths).map_err(|error| {
            ExplorerRequestError::new(format!("Cannot write Windows File Drop clipboard: {error}"))
        })
    }
}

#[cfg(test)]
#[derive(Default)]
struct MemoryFileDropClipboard {
    paths: Mutex<Vec<PathBuf>>,
}

#[cfg(test)]
impl MemoryFileDropClipboard {
    fn with_paths(paths: Vec<PathBuf>) -> Self {
        Self {
            paths: Mutex::new(paths),
        }
    }

    fn paths(&self) -> Vec<PathBuf> {
        self.paths.lock().unwrap().clone()
    }
}

#[cfg(test)]
impl FileDropClipboard for MemoryFileDropClipboard {
    fn read_paths(&self) -> Result<Vec<PathBuf>, ExplorerRequestError> {
        Ok(self.paths())
    }

    fn write_paths(&self, paths: &[PathBuf]) -> Result<(), ExplorerRequestError> {
        *self.paths.lock().unwrap() = paths.to_vec();
        Ok(())
    }
}

fn build_file_drop_payload(paths: &[&Path]) -> io::Result<Vec<u8>> {
    if paths.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "File Drop clipboard requires at least one path",
        ));
    }

    let mut payload = vec![0u8; 20];
    payload[0..4].copy_from_slice(&20u32.to_le_bytes());
    payload[16..20].copy_from_slice(&1i32.to_le_bytes());
    for path in paths {
        let encoded = encode_path_wide(path);
        if encoded.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "clipboard path contains a NUL character: {}",
                    path.display()
                ),
            ));
        }
        for unit in encoded {
            payload.extend_from_slice(&unit.to_le_bytes());
        }
        payload.extend_from_slice(&0u16.to_le_bytes());
    }
    payload.extend_from_slice(&0u16.to_le_bytes());
    Ok(payload)
}

#[cfg(windows)]
fn encode_path_wide(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().collect()
}

#[cfg(not(windows))]
fn encode_path_wide(path: &Path) -> Vec<u16> {
    path.to_string_lossy().encode_utf16().collect()
}

fn prepare_request_with_clipboard<C: FileDropClipboard>(
    mut request: ExplorerRequest,
    clipboard: &C,
) -> Result<ExplorerRequest, ExplorerRequestError> {
    match request.action {
        ExplorerAction::Copy => {
            request.paths = dedupe_paths(request.paths)?;
            clipboard.write_paths(&request.paths)?;
        }
        ExplorerAction::Paste => {
            let clipboard_paths = clipboard.read_paths()?;
            if clipboard_paths.is_empty() {
                return Err(ExplorerRequestError::new(
                    "Windows clipboard does not contain any File Drop paths",
                ));
            }
            if clipboard_paths.len() > MAX_EXPLORER_PATHS {
                return Err(ExplorerRequestError::new(
                    "Windows File Drop clipboard cannot contain more than 100 paths",
                ));
            }
            let selection = SourceSelection::from_paths(clipboard_paths)
                .map_err(|error| ExplorerRequestError::new(error.to_string()))?;
            let destination = request.paths.first().cloned().ok_or_else(|| {
                ExplorerRequestError::new("Explorer paste destination is missing")
            })?;
            validate_paste_destination_with_probe(
                &selection,
                &destination,
                probe_destination_writable,
            )?;
            request.source_selection = Some(selection);
            request.destination_path = Some(destination);
        }
        ExplorerAction::SetSource => {
            request.paths = dedupe_paths(request.paths)?;
            let is_file = request.paths.len() == 1
                && fs::metadata(&request.paths[0])
                    .map(|metadata| metadata.is_file())
                    .unwrap_or(false);
            if request.paths.len() > 1 || is_file {
                request.source_selection = Some(
                    SourceSelection::from_paths(request.paths.clone())
                        .map_err(|error| ExplorerRequestError::new(error.to_string()))?,
                );
            }
        }
        ExplorerAction::SetDestination => {}
    }
    Ok(request)
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>, ExplorerRequestError> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for path in paths {
        if !path.is_absolute() {
            return Err(ExplorerRequestError::new(format!(
                "Explorer path must be absolute: {}",
                path.display()
            )));
        }
        let canonical = fs::canonicalize(&path).map_err(|error| {
            ExplorerRequestError::new(format!(
                "Cannot resolve Explorer path {}: {error}",
                path.display()
            ))
        })?;
        let key = path_key(&canonical);
        if seen.insert(key) {
            deduped.push(path);
        }
    }
    if deduped.len() > MAX_EXPLORER_PATHS {
        return Err(ExplorerRequestError::new(
            "Explorer request cannot contain more than 100 unique paths",
        ));
    }
    Ok(deduped)
}

fn validate_paste_destination_with_probe<F>(
    selection: &SourceSelection,
    destination: &Path,
    writable_probe: F,
) -> Result<(), ExplorerRequestError>
where
    F: FnOnce(&Path) -> io::Result<()>,
{
    let metadata = fs::metadata(destination).map_err(|error| {
        ExplorerRequestError::new(format!(
            "Cannot inspect Explorer paste destination {}: {error}",
            destination.display()
        ))
    })?;
    if !metadata.is_dir() {
        return Err(ExplorerRequestError::new(
            "Explorer paste destination must be a directory",
        ));
    }
    reject_filesystem_links(destination)?;
    let destination = fs::canonicalize(destination).map_err(|error| {
        ExplorerRequestError::new(format!(
            "Cannot resolve Explorer paste destination: {error}"
        ))
    })?;
    for selected in selection.selected_paths() {
        let selected = fs::canonicalize(selected).map_err(|error| {
            ExplorerRequestError::new(format!("Cannot resolve selected source path: {error}"))
        })?;
        if paths_overlap(&selected, &destination) {
            return Err(ExplorerRequestError::new(
                "Explorer paste source and destination must not overlap",
            ));
        }
    }
    writable_probe(&destination).map_err(|error| {
        ExplorerRequestError::new(format!(
            "Explorer paste destination is not writable: {error}"
        ))
    })?;
    Ok(())
}

fn probe_destination_writable(destination: &Path) -> io::Result<()> {
    let probe = destination.join(format!(".offloadkit-write-probe-{}", uuid::Uuid::new_v4()));
    let result = (|| {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe)?;
        file.sync_all()?;
        drop(file);
        fs::remove_file(&probe)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&probe);
    }
    result
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    let left = path_key(left);
    let right = path_key(right);
    left == right
        || left.starts_with(&format!("{right}\\"))
        || right.starts_with(&format!("{left}\\"))
}

#[cfg(windows)]
fn path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

#[cfg(not(windows))]
fn path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_owned()
}

fn explorer_action_requires_focus(action: &ExplorerAction) -> bool {
    !matches!(action, ExplorerAction::Copy)
}

pub fn prepare_explorer_activation(activation: ExplorerActivation) -> ExplorerActivation {
    match activation {
        ExplorerActivation::Request(request) => {
            let id = request.id.clone();
            match prepare_request_with_clipboard(request, &NativeFileDropClipboard) {
                Ok(request) => ExplorerActivation::Request(request),
                Err(error) => ExplorerActivation::Error(ExplorerErrorPayload {
                    id,
                    message: error.to_string(),
                }),
            }
        }
        other => other,
    }
}

#[cfg(windows)]
mod native_file_drop {
    use super::build_file_drop_payload;
    use std::ffi::{c_void, OsString};
    use std::io;
    use std::os::windows::ffi::OsStringExt;
    use std::path::{Path, PathBuf};
    use std::ptr;
    use std::time::Duration;

    type Handle = *mut c_void;
    const CF_HDROP: u32 = 15;
    const GMEM_MOVEABLE: u32 = 0x0002;
    const GMEM_ZEROINIT: u32 = 0x0040;
    const DRAG_QUERY_FILE_COUNT: u32 = 0xFFFF_FFFF;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn OpenClipboard(owner: Handle) -> i32;
        fn CloseClipboard() -> i32;
        fn EmptyClipboard() -> i32;
        fn SetClipboardData(format: u32, memory: Handle) -> Handle;
        fn GetClipboardData(format: u32) -> Handle;
        fn IsClipboardFormatAvailable(format: u32) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalAlloc(flags: u32, bytes: usize) -> Handle;
        fn GlobalLock(memory: Handle) -> *mut c_void;
        fn GlobalUnlock(memory: Handle) -> i32;
        fn GlobalFree(memory: Handle) -> Handle;
    }

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn DragQueryFileW(
            drop_handle: Handle,
            file_index: u32,
            file_name: *mut u16,
            file_name_size: u32,
        ) -> u32;
    }

    struct ClipboardGuard;

    impl ClipboardGuard {
        fn open() -> io::Result<Self> {
            for _ in 0..10 {
                if unsafe { OpenClipboard(ptr::null_mut()) } != 0 {
                    return Ok(Self);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(io::Error::last_os_error())
        }
    }

    impl Drop for ClipboardGuard {
        fn drop(&mut self) {
            unsafe {
                CloseClipboard();
            }
        }
    }

    struct GlobalMemory(Handle);

    impl Drop for GlobalMemory {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    GlobalFree(self.0);
                }
            }
        }
    }

    pub fn write_paths(paths: &[PathBuf]) -> io::Result<()> {
        let path_refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
        let payload = build_file_drop_payload(&path_refs)?;
        let _clipboard = ClipboardGuard::open()?;
        if unsafe { EmptyClipboard() } == 0 {
            return Err(io::Error::last_os_error());
        }

        let memory =
            GlobalMemory(unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, payload.len()) });
        if memory.0.is_null() {
            return Err(io::Error::last_os_error());
        }
        let target = unsafe { GlobalLock(memory.0) };
        if target.is_null() {
            return Err(io::Error::last_os_error());
        }
        unsafe {
            ptr::copy_nonoverlapping(payload.as_ptr(), target.cast::<u8>(), payload.len());
            GlobalUnlock(memory.0);
        }

        if unsafe { SetClipboardData(CF_HDROP, memory.0) }.is_null() {
            return Err(io::Error::last_os_error());
        }
        std::mem::forget(memory);
        Ok(())
    }

    pub fn read_paths() -> io::Result<Vec<PathBuf>> {
        let _clipboard = ClipboardGuard::open()?;
        if unsafe { IsClipboardFormatAvailable(CF_HDROP) } == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "clipboard does not contain Windows File Drop data",
            ));
        }
        let handle = unsafe { GetClipboardData(CF_HDROP) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        let count = unsafe { DragQueryFileW(handle, DRAG_QUERY_FILE_COUNT, ptr::null_mut(), 0) };
        let mut paths = Vec::with_capacity(count as usize);
        for index in 0..count {
            let length = unsafe { DragQueryFileW(handle, index, ptr::null_mut(), 0) };
            let mut buffer = vec![0u16; length as usize + 1];
            let written =
                unsafe { DragQueryFileW(handle, index, buffer.as_mut_ptr(), buffer.len() as u32) };
            if written == 0 && length != 0 {
                return Err(io::Error::last_os_error());
            }
            buffer.truncate(written as usize);
            paths.push(PathBuf::from(OsString::from_wide(&buffer)));
        }
        Ok(paths)
    }
}

#[cfg(not(windows))]
mod native_file_drop {
    use std::io;
    use std::path::PathBuf;

    pub fn write_paths(_paths: &[PathBuf]) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Windows File Drop clipboard is only available on Windows",
        ))
    }

    pub fn read_paths() -> io::Result<Vec<PathBuf>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Windows File Drop clipboard is only available on Windows",
        ))
    }
}

pub fn parse_explorer_request<I>(args: I) -> Result<ExplorerRequest, ExplorerRequestError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let _executable = args.next();
    let remaining: Vec<OsString> = args.collect();
    let mut action = None;
    let mut paths = Vec::new();
    let mut index = 0;

    while index < remaining.len() {
        match remaining[index].as_os_str() {
            value if value == OsStr::new("--explorer-action") => {
                if action.is_some() {
                    return Err(ExplorerRequestError::new(
                        "Explorer action may only be specified once",
                    ));
                }
                let value = remaining
                    .get(index + 1)
                    .ok_or_else(|| ExplorerRequestError::new("Explorer action value is missing"))?;
                action = Some(parse_action(value)?);
                index += 2;
            }
            value if value == OsStr::new("--path") => {
                let value = remaining
                    .get(index + 1)
                    .ok_or_else(|| ExplorerRequestError::new("Explorer path value is missing"))?;
                if value.is_empty() {
                    return Err(ExplorerRequestError::new("Explorer path cannot be empty"));
                }
                paths.push(PathBuf::from(value));
                if paths.len() > MAX_EXPLORER_PATHS {
                    return Err(ExplorerRequestError::new(
                        "Explorer request cannot contain more than 100 paths",
                    ));
                }
                index += 2;
            }
            value => {
                return Err(ExplorerRequestError::new(format!(
                    "Unknown Explorer argument: {}",
                    value.to_string_lossy()
                )));
            }
        }
    }

    let action = action.ok_or_else(|| ExplorerRequestError::new("Explorer action is missing"))?;
    validate_paths(&action, &paths)?;

    Ok(ExplorerRequest {
        id: uuid::Uuid::new_v4().to_string(),
        action,
        paths,
        source_selection: None,
        destination_path: None,
    })
}

pub fn parse_explorer_activation<I>(args: I) -> ExplorerActivation
where
    I: IntoIterator<Item = OsString>,
{
    let args: Vec<OsString> = args.into_iter().collect();
    let is_explorer_invocation = args
        .iter()
        .any(|arg| arg == OsStr::new("--explorer-action") || arg == OsStr::new("--path"));
    if !is_explorer_invocation {
        return ExplorerActivation::None;
    }

    match parse_explorer_request(args) {
        Ok(request) => ExplorerActivation::Request(request),
        Err(error) => ExplorerActivation::Error(ExplorerErrorPayload {
            id: uuid::Uuid::new_v4().to_string(),
            message: error.to_string(),
        }),
    }
}

#[cfg(windows)]
pub fn handle_secondary_instance(app: &AppHandle, args: Vec<String>) {
    let activation = parse_explorer_activation(args.into_iter().map(OsString::from));
    let activation = match activation {
        ExplorerActivation::Request(request) => {
            ExplorerActivation::Request(app.state::<ExplorerCopyAggregationState>().merge(request))
        }
        other => other,
    };
    let activation = prepare_explorer_activation(activation);
    if matches!(activation, ExplorerActivation::None) {
        return;
    }

    let requires_focus = match &activation {
        ExplorerActivation::Request(request) => explorer_action_requires_focus(&request.action),
        ExplorerActivation::Error(_) => true,
        ExplorerActivation::None => false,
    };
    if requires_focus {
        focus_main_window(app);
    }
    let state = app.state::<ExplorerPendingState>();
    let events = state.enqueue(activation);
    if let Err(error) = emit_events(app, &state, events) {
        eprintln!("Failed to forward Explorer activation: {error}");
    }
}

#[tauri::command]
pub fn explorer_frontend_ready(
    app: AppHandle,
    state: State<'_, ExplorerPendingState>,
) -> Result<(), String> {
    let events = state.mark_frontend_ready();
    emit_events(&app, &state, events)
}

#[tauri::command]
pub fn acknowledge_explorer_request(
    request_id: String,
    state: State<'_, ExplorerPendingState>,
) -> Result<(), String> {
    if state.acknowledge(&request_id) {
        Ok(())
    } else {
        Err(format!(
            "Explorer request is not awaiting acknowledgement: {request_id}"
        ))
    }
}

fn emit_events(
    app: &AppHandle,
    state: &ExplorerPendingState,
    events: Vec<ExplorerEvent>,
) -> Result<(), String> {
    for (index, event) in events.iter().enumerate() {
        let result = match event {
            ExplorerEvent::Request(request) => app.emit(EXPLORER_REQUEST_EVENT, request),
            ExplorerEvent::Error(error) => app.emit(EXPLORER_ERROR_EVENT, error),
        };
        if let Err(error) = result {
            for unreported in events[index..].iter().rev() {
                state.requeue_after_emit_failure(unreported.clone());
            }
            return Err(error.to_string());
        }
    }
    Ok(())
}

#[cfg(windows)]
fn focus_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

fn parse_action(value: &OsStr) -> Result<ExplorerAction, ExplorerRequestError> {
    if value == OsStr::new("set-source") {
        Ok(ExplorerAction::SetSource)
    } else if value == OsStr::new("set-destination") {
        Ok(ExplorerAction::SetDestination)
    } else if value == OsStr::new("copy") {
        Ok(ExplorerAction::Copy)
    } else if value == OsStr::new("paste") {
        Ok(ExplorerAction::Paste)
    } else {
        Err(ExplorerRequestError::new("Unknown Explorer action"))
    }
}

fn validate_paths(action: &ExplorerAction, paths: &[PathBuf]) -> Result<(), ExplorerRequestError> {
    if paths.is_empty() {
        return Err(ExplorerRequestError::new(
            "Explorer request requires at least one path",
        ));
    }
    if matches!(
        action,
        ExplorerAction::SetDestination | ExplorerAction::Paste
    ) && paths.len() != 1
    {
        return Err(ExplorerRequestError::new(
            "Explorer destination or paste action requires exactly one path",
        ));
    }

    for path in paths {
        let metadata = fs::metadata(path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                ExplorerRequestError::new(format!(
                    "Explorer path does not exist: {}",
                    path.display()
                ))
            } else {
                ExplorerRequestError::new(format!(
                    "Cannot inspect Explorer path {}: {error}",
                    path.display()
                ))
            }
        })?;
        reject_filesystem_links(path)?;
        if matches!(
            action,
            ExplorerAction::SetDestination | ExplorerAction::Paste
        ) && !metadata.is_dir()
        {
            return Err(ExplorerRequestError::new(
                "Explorer destination path must be a directory",
            ));
        }
    }

    Ok(())
}

fn reject_filesystem_links(path: &Path) -> Result<(), ExplorerRequestError> {
    for component_path in path.ancestors() {
        let metadata = fs::symlink_metadata(component_path).map_err(|error| {
            ExplorerRequestError::new(format!(
                "Cannot inspect Explorer path {}: {error}",
                component_path.display()
            ))
        })?;
        if metadata.file_type().is_symlink() || is_windows_reparse_point(&metadata) {
            return Err(ExplorerRequestError::new(format!(
                "Filesystem link or reparse point is not allowed: {}",
                component_path.display()
            )));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn is_windows_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn is_windows_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(test)]
#[derive(Default)]
struct MemoryRegistry {
    values: Mutex<std::collections::HashMap<(String, String), String>>,
    fail_on_write: Option<usize>,
    writes: Mutex<usize>,
}
#[cfg(test)]
impl MemoryRegistry {
    fn failing_on_write(write: usize) -> Self {
        Self {
            fail_on_write: Some(write),
            ..Self::default()
        }
    }
}
#[cfg(test)]
impl RegistryAccess for MemoryRegistry {
    fn key_exists(&self, key: &str) -> Result<bool, RegistryAccessError> {
        Ok(self
            .values
            .lock()
            .unwrap()
            .keys()
            .any(|(candidate, _)| candidate == key || candidate.starts_with(&format!("{key}\\"))))
    }
    fn create_key(&self, _key: &str) -> Result<(), RegistryAccessError> {
        Ok(())
    }
    fn set_string(
        &self,
        key: &str,
        name: Option<&str>,
        value: &str,
    ) -> Result<(), RegistryAccessError> {
        let mut writes = self.writes.lock().unwrap();
        *writes += 1;
        if self.fail_on_write == Some(*writes) {
            return Err(RegistryAccessError::Other("simulated failure".into()));
        }
        self.values
            .lock()
            .unwrap()
            .insert((key.into(), name.unwrap_or("").into()), value.into());
        Ok(())
    }
    fn get_string(&self, key: &str, name: Option<&str>) -> Result<String, RegistryAccessError> {
        self.values
            .lock()
            .unwrap()
            .get(&(key.into(), name.unwrap_or("").into()))
            .cloned()
            .ok_or(RegistryAccessError::NotFound)
    }
    fn delete_tree(&self, key: &str) -> Result<(), RegistryAccessError> {
        self.values.lock().unwrap().retain(|(candidate, _), _| {
            candidate != key && !candidate.starts_with(&format!("{key}\\"))
        });
        Ok(())
    }
}

#[cfg(all(test, windows))]
struct TestRegistryNamespace {
    registry: WindowsRegistry,
    scope: RegistryScope,
}
#[cfg(all(test, windows))]
impl TestRegistryNamespace {
    fn new() -> Self {
        Self {
            registry: WindowsRegistry,
            scope: RegistryScope::new(format!(
                r"Software\OffloadKit\Phase13ATest\{}",
                uuid::Uuid::new_v4()
            )),
        }
    }
}
#[cfg(all(test, windows))]
impl Drop for TestRegistryNamespace {
    fn drop(&mut self) {
        let _ = RegistryAccess::delete_tree(&self.registry, &self.scope.base);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_file_drop_payload, build_registry_command, explorer_action_requires_focus,
        explorer_verbs, install_with_registry, integration_status_with_registry,
        parse_explorer_activation, parse_explorer_request, prepare_request_with_clipboard,
        uninstall_with_registry, validate_paste_destination_with_probe, ExplorerAction,
        ExplorerActivation, ExplorerCopyAggregationState, ExplorerEvent, ExplorerPendingState,
        MemoryFileDropClipboard, MemoryRegistry, NativeFileDropClipboard, RegistryAccess,
        RegistryScope, TestRegistryNamespace,
    };
    use std::ffi::OsString;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    fn explorer_args(action: &str, paths: &[&Path]) -> Vec<OsString> {
        let mut args = vec![
            OsString::from("OffloadKit.exe"),
            OsString::from("--explorer-action"),
            OsString::from(action),
        ];
        for path in paths {
            args.push(OsString::from("--path"));
            args.push(path.as_os_str().to_owned());
        }
        args
    }

    #[test]
    fn parses_set_source_with_unicode_and_spaces_exactly() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("Adobe Premiere Pro Tự động lưu");
        fs::create_dir(&path).unwrap();

        let request = parse_explorer_request(explorer_args("set-source", &[&path])).unwrap();

        assert_eq!(request.action, ExplorerAction::SetSource);
        assert_eq!(request.paths, vec![path]);
        assert!(!request.id.is_empty());
    }

    #[test]
    fn parses_set_destination() {
        let temp = tempdir().unwrap();

        let request =
            parse_explorer_request(explorer_args("set-destination", &[temp.path()])).unwrap();

        assert_eq!(request.action, ExplorerAction::SetDestination);
        assert_eq!(request.paths, vec![temp.path().to_path_buf()]);
    }

    #[test]
    fn parses_copy_and_paste_actions() {
        let temp = tempdir().unwrap();
        let file = temp.path().join("A001.mov");
        fs::write(&file, b"clip").unwrap();

        let copy = parse_explorer_request(explorer_args("copy", &[&file])).unwrap();
        let paste = parse_explorer_request(explorer_args("paste", &[temp.path()])).unwrap();

        assert_eq!(copy.action, ExplorerAction::Copy);
        assert_eq!(paste.action, ExplorerAction::Paste);
    }

    #[test]
    fn copy_preparation_deduplicates_paths_and_writes_file_drop_without_a_selection_payload() {
        let temp = tempdir().unwrap();
        let file = temp.path().join("A001.mov");
        fs::write(&file, b"clip").unwrap();
        let clipboard = MemoryFileDropClipboard::default();
        let request = parse_explorer_request(explorer_args("copy", &[&file, &file])).unwrap();

        let prepared = prepare_request_with_clipboard(request, &clipboard).unwrap();

        assert_eq!(prepared.action, ExplorerAction::Copy);
        assert_eq!(prepared.paths, vec![file.clone()]);
        assert!(prepared.source_selection.is_none());
        assert_eq!(clipboard.paths(), vec![file]);
    }

    #[test]
    fn paste_preparation_reads_file_drop_prunes_nested_paths_and_sets_destination() {
        let source = tempdir().unwrap();
        let destination = tempdir().unwrap();
        let folder = source.path().join("CARD_A");
        let nested = folder.join("DCIM").join("A001.mov");
        fs::create_dir_all(nested.parent().unwrap()).unwrap();
        fs::write(&nested, b"clip").unwrap();
        let clipboard = MemoryFileDropClipboard::with_paths(vec![folder.clone(), nested]);
        let request =
            parse_explorer_request(explorer_args("paste", &[destination.path()])).unwrap();

        let prepared = prepare_request_with_clipboard(request, &clipboard).unwrap();

        assert_eq!(prepared.action, ExplorerAction::Paste);
        assert_eq!(
            prepared.destination_path.as_deref(),
            Some(destination.path())
        );
        let selection = prepared.source_selection.unwrap();
        assert_eq!(selection.common_root(), source.path());
        assert_eq!(selection.selected_paths(), &[folder]);
    }

    #[test]
    fn paste_preparation_rejects_empty_file_drop_without_producing_a_request() {
        let destination = tempdir().unwrap();
        let clipboard = MemoryFileDropClipboard::default();
        let request =
            parse_explorer_request(explorer_args("paste", &[destination.path()])).unwrap();

        let error = prepare_request_with_clipboard(request, &clipboard).unwrap_err();

        assert!(error.to_string().contains("empty") || error.to_string().contains("File Drop"));
    }

    #[test]
    fn paste_destination_rejects_overlap_and_a_failed_write_probe() {
        let source = tempdir().unwrap();
        let selected = source.path().join("CARD_A");
        let nested_destination = selected.join("backup");
        fs::create_dir_all(&nested_destination).unwrap();
        let selection = crate::source_selection::SourceSelection::new(
            source.path().to_path_buf(),
            vec![selected],
        )
        .unwrap();

        let overlap =
            validate_paste_destination_with_probe(&selection, &nested_destination, |_| Ok(()))
                .unwrap_err();
        let destination = tempdir().unwrap();
        let unwritable =
            validate_paste_destination_with_probe(&selection, destination.path(), |_| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "simulated read-only destination",
                ))
            })
            .unwrap_err();

        assert!(overlap.to_string().contains("overlap"));
        assert!(unwritable.to_string().contains("writable"));
    }

    #[test]
    fn file_drop_payload_is_utf16_double_nul_terminated() {
        let paths = [Path::new(r"C:\Footage Việt\A001.mov")];
        let payload = build_file_drop_payload(&paths).unwrap();

        assert_eq!(u32::from_le_bytes(payload[0..4].try_into().unwrap()), 20);
        assert_eq!(i32::from_le_bytes(payload[16..20].try_into().unwrap()), 1);
        assert_eq!(&payload[payload.len() - 4..], &[0, 0, 0, 0]);
    }

    #[test]
    fn copy_is_the_only_valid_action_that_does_not_require_window_focus() {
        assert!(!explorer_action_requires_focus(&ExplorerAction::Copy));
        assert!(explorer_action_requires_focus(&ExplorerAction::Paste));
        assert!(explorer_action_requires_focus(&ExplorerAction::SetSource));
        assert!(explorer_action_requires_focus(
            &ExplorerAction::SetDestination
        ));
    }

    #[test]
    fn rapid_copy_invocations_aggregate_and_later_copy_starts_a_new_selection() {
        let temp = tempdir().unwrap();
        let first_path = temp.path().join("A001.mov");
        let second_path = temp.path().join("A002.mov");
        fs::write(&first_path, b"one").unwrap();
        fs::write(&second_path, b"two").unwrap();
        let first = parse_explorer_request(explorer_args("copy", &[&first_path])).unwrap();
        let second = parse_explorer_request(explorer_args("copy", &[&second_path])).unwrap();
        let later = parse_explorer_request(explorer_args("copy", &[&second_path])).unwrap();
        let state = ExplorerCopyAggregationState::default();
        let started = std::time::Instant::now();

        let first = state.merge_at(first, started);
        let combined = state.merge_at(second, started + std::time::Duration::from_millis(50));
        let reset = state.merge_at(later, started + std::time::Duration::from_secs(1));

        assert_eq!(first.paths, vec![first_path.clone()]);
        assert_eq!(combined.paths, vec![first_path, second_path.clone()]);
        assert_eq!(reset.paths, vec![second_path]);
    }

    #[test]
    fn rejects_missing_path_argument() {
        let args = vec![
            OsString::from("OffloadKit.exe"),
            OsString::from("--explorer-action"),
            OsString::from("set-source"),
        ];

        let error = parse_explorer_request(args).unwrap_err();

        assert!(error.to_string().contains("path"));
    }

    #[test]
    fn rejects_unknown_action() {
        let temp = tempdir().unwrap();

        let error = parse_explorer_request(explorer_args("launch", &[temp.path()])).unwrap_err();

        assert!(error.to_string().contains("action"));
    }

    #[test]
    fn accepts_repeated_paths_up_to_one_hundred() {
        let temp = tempdir().unwrap();
        let paths = vec![temp.path(); 100];

        let request = parse_explorer_request(explorer_args("set-source", &paths)).unwrap();

        assert_eq!(request.paths.len(), 100);
    }

    #[test]
    fn rejects_more_than_one_hundred_paths() {
        let temp = tempdir().unwrap();
        let paths = vec![temp.path(); 101];

        let error = parse_explorer_request(explorer_args("set-source", &paths)).unwrap_err();

        assert!(error.to_string().contains("100"));
    }

    #[test]
    fn rejects_empty_path() {
        let args = vec![
            OsString::from("OffloadKit.exe"),
            OsString::from("--explorer-action"),
            OsString::from("set-source"),
            OsString::from("--path"),
            OsString::new(),
        ];

        let error = parse_explorer_request(args).unwrap_err();

        assert!(error.to_string().contains("empty"));
    }

    #[test]
    fn rejects_nonexistent_path() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("missing");

        let error = parse_explorer_request(explorer_args("set-source", &[&path])).unwrap_err();

        assert!(error.to_string().contains("does not exist"));
    }

    #[test]
    fn destination_requires_exactly_one_directory() {
        let temp = tempdir().unwrap();
        let other = tempdir().unwrap();
        let file = temp.path().join("clip.mov");
        fs::write(&file, b"test").unwrap();

        let file_error =
            parse_explorer_request(explorer_args("set-destination", &[&file])).unwrap_err();
        let count_error = parse_explorer_request(explorer_args(
            "set-destination",
            &[temp.path(), other.path()],
        ))
        .unwrap_err();

        assert!(file_error.to_string().contains("directory"));
        assert!(count_error.to_string().contains("exactly one"));
    }

    #[test]
    fn rejects_filesystem_links() {
        let temp = tempdir().unwrap();
        let target = temp.path().join("target");
        let link = temp.path().join("link");
        fs::create_dir(&target).unwrap();
        create_dir_link(&target, &link);

        let error = parse_explorer_request(explorer_args("set-source", &[&link])).unwrap_err();

        assert!(error.to_string().contains("link") || error.to_string().contains("reparse"));
    }

    #[test]
    fn non_explorer_startup_args_produce_no_event() {
        let activation = parse_explorer_activation([
            OsString::from("OffloadKit.exe"),
            OsString::from("--ordinary-flag"),
        ]);

        assert_eq!(activation, ExplorerActivation::None);
    }

    #[test]
    fn valid_secondary_args_produce_one_request_event() {
        let temp = tempdir().unwrap();

        let activation = parse_explorer_activation(explorer_args("set-source", &[temp.path()]));
        let state = ExplorerPendingState::default();
        assert!(state.mark_frontend_ready().is_empty());
        let events = state.enqueue(activation);

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], ExplorerEvent::Request(_)));
    }

    #[test]
    fn duplicate_request_id_is_never_dispatched_twice() {
        let temp = tempdir().unwrap();
        let request = parse_explorer_request(explorer_args("set-source", &[temp.path()])).unwrap();
        let state = ExplorerPendingState::default();
        state.mark_frontend_ready();

        let first = state.enqueue(ExplorerActivation::Request(request.clone()));
        let duplicate_while_in_flight = state.enqueue(ExplorerActivation::Request(request.clone()));
        assert!(state.acknowledge(&request.id));
        let duplicate_after_ack = state.enqueue(ExplorerActivation::Request(request));

        assert_eq!(first.len(), 1);
        assert!(duplicate_while_in_flight.is_empty());
        assert!(duplicate_after_ack.is_empty());
    }

    #[test]
    fn invalid_secondary_args_produce_error_without_pending_request() {
        let activation = parse_explorer_activation([
            OsString::from("OffloadKit.exe"),
            OsString::from("--explorer-action"),
            OsString::from("copy"),
        ]);
        let state = ExplorerPendingState::default();
        state.mark_frontend_ready();

        let events = state.enqueue(activation);

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], ExplorerEvent::Error(_)));
        assert_eq!(state.pending_request_count(), 0);
        assert_eq!(state.in_flight_request_count(), 0);
    }

    #[test]
    fn startup_request_waits_for_ready_and_acknowledgement() {
        let temp = tempdir().unwrap();
        let activation =
            parse_explorer_activation(explorer_args("set-destination", &[temp.path()]));
        let request_id = match &activation {
            ExplorerActivation::Request(request) => request.id.clone(),
            _ => panic!("expected request"),
        };
        let state = ExplorerPendingState::new(activation);

        assert_eq!(state.pending_request_count(), 1);
        let events = state.mark_frontend_ready();
        assert_eq!(events.len(), 1);
        assert_eq!(state.in_flight_request_count(), 1);
        assert!(state.acknowledge(&request_id));
        assert_eq!(state.in_flight_request_count(), 0);
    }

    #[test]
    fn registry_command_quotes_spaces_ampersand_unicode_and_quotes() {
        let executable = Path::new(r#"C:\Program Files\OffloadKit Việt & Test\Offload"Kit.exe"#);

        let command = build_registry_command(executable, &ExplorerAction::SetSource).unwrap();

        assert_eq!(
            command,
            r#""C:\Program Files\OffloadKit Việt & Test\Offload\"Kit.exe" --explorer-action set-source --path "%V""#
        );
    }

    #[test]
    fn registry_defines_copy_for_files_and_folders_and_paste_for_folder_targets() {
        let keys: Vec<&str> = explorer_verbs().iter().map(|verb| verb.key).collect();

        assert_eq!(keys.len(), 9);
        assert!(keys.contains(&r"Software\Classes\*\shell\OffloadKit.Copy"));
        assert!(keys.contains(&r"Software\Classes\Directory\shell\OffloadKit.Copy"));
        assert!(keys.contains(&r"Software\Classes\Directory\shell\OffloadKit.Paste"));
        assert!(keys.contains(&r"Software\Classes\Directory\Background\shell\OffloadKit.Paste"));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "replaces the real Windows clipboard and leaves an inspectable temp smoke tree"]
    fn phase13b_real_windows_file_drop_and_selected_copy_smoke() {
        use crate::checksum::{hash_file, ChecksumAlgorithm};
        use crate::copy_engine::{
            run_copy_core_with_selection, ProgressPayload, ProgressSink, VerificationMode,
        };
        use crate::organize::OrganizeSettings;
        use std::sync::atomic::AtomicBool;

        struct NoopSink;
        impl ProgressSink for NoopSink {
            fn on_scan(&self, _total_files: u64, _total_bytes: u64) {}
            fn on_progress(&self, _payload: ProgressPayload) {}
        }

        let smoke_root = std::env::temp_dir().join(format!(
            "OffloadKit-Phase13B-Smoke-{}",
            uuid::Uuid::new_v4()
        ));
        let source = smoke_root.join("source");
        let destination = smoke_root.join("destination");
        let selected_folder = source.join("CARD_A");
        let selected_video = selected_folder.join("DCIM").join("A001.mov");
        let selected_audio = source.join("AUDIO").join("A001.wav");
        let unselected = source.join("PRIVATE").join("leave.txt");
        fs::create_dir_all(selected_video.parent().unwrap()).unwrap();
        fs::create_dir_all(selected_audio.parent().unwrap()).unwrap();
        fs::create_dir_all(unselected.parent().unwrap()).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(&selected_video, b"phase13b selected video bytes").unwrap();
        fs::write(&selected_audio, b"phase13b selected audio bytes").unwrap();
        fs::write(&unselected, b"must remain source-only").unwrap();

        let copy =
            parse_explorer_request(explorer_args("copy", &[&selected_folder, &selected_audio]))
                .unwrap();
        let copy = prepare_request_with_clipboard(copy, &NativeFileDropClipboard).unwrap();
        assert_eq!(copy.paths.len(), 2);

        let paste = parse_explorer_request(explorer_args("paste", &[&destination])).unwrap();
        let paste = prepare_request_with_clipboard(paste, &NativeFileDropClipboard).unwrap();
        let selection = paste.source_selection.unwrap();
        let outcome = run_copy_core_with_selection(
            &NoopSink,
            "phase13b-real-smoke".to_string(),
            &selection,
            &destination,
            &AtomicBool::new(false),
            crate::copy_engine::CopyOptions::new(
                VerificationMode::SourceAndDestination,
                ChecksumAlgorithm::Sha1,
                "Phase13B Smoke",
                &OrganizeSettings::default(),
                false,
                false,
                None,
            ),
        );

        let destination_video = destination.join("CARD_A").join("DCIM").join("A001.mov");
        let destination_audio = destination.join("AUDIO").join("A001.wav");
        let source_video_hash = hash_file(&selected_video, ChecksumAlgorithm::Sha1).unwrap();
        let destination_video_hash =
            hash_file(&destination_video, ChecksumAlgorithm::Sha1).unwrap();
        let source_audio_hash = hash_file(&selected_audio, ChecksumAlgorithm::Sha1).unwrap();
        let destination_audio_hash =
            hash_file(&destination_audio, ChecksumAlgorithm::Sha1).unwrap();

        assert!(!outcome.cancelled);
        assert!(outcome.failed_files.is_empty());
        assert_eq!(outcome.files_copied, 2);
        assert_eq!(outcome.verified_files.len(), 2);
        assert_eq!(outcome.mhl_entries.len(), 2);
        assert!(outcome.missing_files.is_empty());
        assert_eq!(source_video_hash, destination_video_hash);
        assert_eq!(source_audio_hash, destination_audio_hash);
        assert!(selected_video.exists());
        assert!(selected_audio.exists());
        assert!(unselected.exists());
        assert!(!destination.join("PRIVATE").exists());

        println!("SMOKE_ROOT={}", smoke_root.display());
        println!("SOURCE_VIDEO={}", selected_video.display());
        println!("DESTINATION_VIDEO={}", destination_video.display());
        println!("VIDEO_SHA1={source_video_hash}");
        println!("SOURCE_AUDIO={}", selected_audio.display());
        println!("DESTINATION_AUDIO={}", destination_audio.display());
        println!("AUDIO_SHA1={source_audio_hash}");
        println!("SOURCE_PRESERVED=true");
        println!("UNSELECTED_DESTINATION_ABSENT=true");
    }

    #[cfg(windows)]
    #[test]
    fn registry_install_writes_all_verbs_and_reads_back_healthy() {
        let namespace = TestRegistryNamespace::new();
        let executable = Path::new(r"C:\Program Files\OffloadKit Việt & Test\OffloadKit.exe");

        let installed = install_with_registry(&namespace.registry, &namespace.scope, executable)
            .expect("install should succeed");
        let status =
            integration_status_with_registry(&namespace.registry, &namespace.scope, executable)
                .expect("status should succeed");

        assert!(installed.installed);
        assert!(status.healthy);
        assert_eq!(status.matching_commands, 9);
        for verb in explorer_verbs() {
            let command_key = namespace.scope.qualify(&format!("{}\\command", verb.key));
            let command = namespace.registry.get_string(&command_key, None).unwrap();
            assert_eq!(
                command,
                build_registry_command(executable, &verb.action).unwrap()
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn partial_registry_install_is_unhealthy() {
        let namespace = TestRegistryNamespace::new();
        let executable = Path::new(r"C:\OffloadKit\OffloadKit.exe");
        install_with_registry(&namespace.registry, &namespace.scope, executable).unwrap();
        let missing = namespace
            .scope
            .qualify(r"Software\Classes\Directory\shell\OffloadKit.SetDestination\command");
        namespace.registry.delete_tree(&missing).unwrap();

        let status =
            integration_status_with_registry(&namespace.registry, &namespace.scope, executable)
                .unwrap();

        assert!(!status.installed);
        assert!(!status.healthy);
        assert_eq!(status.matching_commands, 8);
    }

    #[test]
    fn failed_registry_install_rolls_back_keys_created_by_that_attempt() {
        let registry = MemoryRegistry::failing_on_write(7);
        let scope = RegistryScope::new("Software\\OffloadKit\\Phase13AFake");
        let executable = Path::new(r"C:\OffloadKit\OffloadKit.exe");

        let error = install_with_registry(&registry, &scope, executable).unwrap_err();

        assert!(error.message.contains("rolled back"));
        for verb in explorer_verbs() {
            assert!(!registry.key_exists(&scope.qualify(verb.key)).unwrap());
        }
    }

    #[cfg(windows)]
    #[test]
    fn registry_uninstall_is_idempotent_and_preserves_unrelated_keys() {
        let namespace = TestRegistryNamespace::new();
        let executable = Path::new(r"C:\OffloadKit\OffloadKit.exe");
        let unrelated = namespace
            .scope
            .qualify(r"Software\Classes\Directory\shell\AnotherApp.Command");
        namespace
            .registry
            .set_string(&unrelated, None, "Keep me")
            .unwrap();
        install_with_registry(&namespace.registry, &namespace.scope, executable).unwrap();

        let first = uninstall_with_registry(&namespace.registry, &namespace.scope, executable)
            .expect("first uninstall should succeed");
        let second = uninstall_with_registry(&namespace.registry, &namespace.scope, executable)
            .expect("second uninstall should succeed");

        assert!(!first.installed);
        assert!(!second.installed);
        assert_eq!(
            namespace.registry.get_string(&unrelated, None).unwrap(),
            "Keep me"
        );
    }

    #[cfg(windows)]
    fn create_dir_link(target: &Path, link: &Path) {
        std::os::windows::fs::symlink_dir(target, link).unwrap();
    }

    #[cfg(unix)]
    fn create_dir_link(target: &Path, link: &Path) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }
}
