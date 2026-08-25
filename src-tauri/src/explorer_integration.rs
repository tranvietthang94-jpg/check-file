use serde::{Deserialize, Serialize};
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::{collections::HashSet, collections::VecDeque};
use tauri::{AppHandle, Emitter, Manager, State};

const MAX_EXPLORER_PATHS: usize = 100;
pub const EXPLORER_REQUEST_EVENT: &str = "explorer://request";
pub const EXPLORER_ERROR_EVENT: &str = "explorer://error";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExplorerAction {
    SetSource,
    SetDestination,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExplorerRequest {
    pub id: String,
    pub action: ExplorerAction,
    pub paths: Vec<PathBuf>,
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

const EXPLORER_VERBS: [ExplorerVerb; 5] = [
    ExplorerVerb {
        key: r"Software\Classes\*\shell\OffloadKit.SetSource",
        label: "Đặt làm Source trong OffloadKit",
        action: ExplorerAction::SetSource,
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
    if matches!(activation, ExplorerActivation::None) {
        return;
    }

    focus_main_window(app);
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
    if matches!(action, ExplorerAction::SetDestination) && paths.len() != 1 {
        return Err(ExplorerRequestError::new(
            "Explorer destination requires exactly one path",
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
        if matches!(action, ExplorerAction::SetDestination) && !metadata.is_dir() {
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
        build_registry_command, explorer_verbs, install_with_registry,
        integration_status_with_registry, parse_explorer_activation, parse_explorer_request,
        uninstall_with_registry, ExplorerAction, ExplorerActivation, ExplorerEvent,
        ExplorerPendingState, MemoryRegistry, RegistryAccess, RegistryScope, TestRegistryNamespace,
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

        let error = parse_explorer_request(explorer_args("copy", &[temp.path()])).unwrap_err();

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
        assert_eq!(status.matching_commands, 5);
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
        assert_eq!(status.matching_commands, 4);
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
