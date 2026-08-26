use crate::explorer_integration::ExplorerAction;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub const PRODUCTION_EXECUTABLE_PATH: &str =
    "/Applications/OffloadKit.app/Contents/MacOS/offloadkit";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedWorkflow {
    pub bundle_name: &'static str,
    pub action: ExplorerAction,
    pub executable_path: PathBuf,
    pub info_plist: String,
    pub document_wflow: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FinderIntegrationStatus {
    pub supported: bool,
    pub installed: bool,
    pub healthy: bool,
    pub misplaced_app: bool,
    pub executable_path: String,
    pub expected_workflows: usize,
    pub installed_workflows: usize,
    pub matching_workflows: usize,
    pub problems: Vec<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FinderIntegrationError {
    pub code: String,
    pub message: String,
}

impl FinderIntegrationError {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

pub fn status_at(
    services_root: &Path,
    executable: &Path,
    require_production_executable: bool,
) -> Result<FinderIntegrationStatus, FinderIntegrationError> {
    let executable_text = executable.to_str().ok_or_else(|| {
        FinderIntegrationError::new(
            "invalidExecutablePath",
            "OffloadKit executable path is not valid Unicode",
        )
    })?;
    let workflows = render_workflows(executable).map_err(|error| {
        FinderIntegrationError::new(
            "workflowRenderFailed",
            format!("Cannot render Finder Quick Actions: {error}"),
        )
    })?;
    let misplaced_app =
        require_production_executable && executable != Path::new(PRODUCTION_EXECUTABLE_PATH);
    let mut problems = Vec::new();
    if misplaced_app {
        problems.push(format!(
            "OffloadKit must run from {PRODUCTION_EXECUTABLE_PATH}"
        ));
    }

    let mut installed_workflows = 0;
    let mut matching_workflows = 0;
    match fs::symlink_metadata(services_root) {
        Ok(metadata) if metadata.file_type().is_symlink() => problems.push(format!(
            "Finder Services directory must not be a filesystem link: {}",
            services_root.display()
        )),
        Ok(metadata) if !metadata.is_dir() => problems.push(format!(
            "Finder Services path is not a directory: {}",
            services_root.display()
        )),
        Ok(_) => {
            for workflow in &workflows {
                let bundle = services_root.join(workflow.bundle_name);
                if fs::symlink_metadata(&bundle).is_ok() {
                    installed_workflows += 1;
                }
                match workflow_matches(&bundle, workflow) {
                    Ok(true) => matching_workflows += 1,
                    Ok(false) => problems.push(format!(
                        "Finder workflow is missing or does not match: {}",
                        workflow.bundle_name
                    )),
                    Err(error) => problems.push(format!(
                        "Cannot inspect Finder workflow {}: {error}",
                        workflow.bundle_name
                    )),
                }
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            for workflow in &workflows {
                problems.push(format!(
                    "Finder workflow is missing: {}",
                    workflow.bundle_name
                ));
            }
        }
        Err(error) => {
            return Err(FinderIntegrationError::new(
                "servicesReadFailed",
                format!(
                    "Cannot inspect Finder Services directory {}: {error}",
                    services_root.display()
                ),
            ));
        }
    }

    let healthy = !misplaced_app && matching_workflows == workflows.len();
    let message = (!problems.is_empty()).then(|| problems.join("; "));
    Ok(FinderIntegrationStatus {
        supported: true,
        installed: installed_workflows > 0,
        healthy,
        misplaced_app,
        executable_path: executable_text.to_owned(),
        expected_workflows: workflows.len(),
        installed_workflows,
        matching_workflows,
        problems,
        message,
    })
}

pub fn install_at(
    services_root: &Path,
    executable: &Path,
    require_production_executable: bool,
) -> Result<FinderIntegrationStatus, FinderIntegrationError> {
    install_at_inner(
        services_root,
        executable,
        require_production_executable,
        None,
    )
}

pub fn uninstall_at(
    services_root: &Path,
    executable: &Path,
    require_production_executable: bool,
) -> Result<FinderIntegrationStatus, FinderIntegrationError> {
    for workflow in render_workflows(executable).map_err(|error| {
        FinderIntegrationError::new(
            "workflowRenderFailed",
            format!("Cannot render Finder Quick Actions: {error}"),
        )
    })? {
        let bundle = services_root.join(workflow.bundle_name);
        if fs::symlink_metadata(&bundle).is_ok() {
            remove_path(&bundle).map_err(|error| {
                FinderIntegrationError::new(
                    "workflowUninstallFailed",
                    format!("Cannot remove {}: {error}", bundle.display()),
                )
            })?;
        }
    }
    status_at(services_root, executable, require_production_executable)
}

#[cfg(test)]
fn install_at_with_failure(
    services_root: &Path,
    executable: &Path,
    require_production_executable: bool,
    fail_after: usize,
) -> Result<FinderIntegrationStatus, FinderIntegrationError> {
    install_at_inner(
        services_root,
        executable,
        require_production_executable,
        Some(fail_after),
    )
}

fn install_at_inner(
    services_root: &Path,
    executable: &Path,
    require_production_executable: bool,
    fail_after: Option<usize>,
) -> Result<FinderIntegrationStatus, FinderIntegrationError> {
    if require_production_executable && executable != Path::new(PRODUCTION_EXECUTABLE_PATH) {
        return Err(FinderIntegrationError::new(
            "misplacedApplication",
            format!(
                "Move OffloadKit.app to /Applications before enabling Finder Quick Actions. Expected executable: {PRODUCTION_EXECUTABLE_PATH}"
            ),
        ));
    }
    ensure_services_root(services_root)?;
    let workflows = render_workflows(executable).map_err(|error| {
        FinderIntegrationError::new(
            "workflowRenderFailed",
            format!("Cannot render Finder Quick Actions: {error}"),
        )
    })?;
    let transaction_id = uuid::Uuid::new_v4();
    let staging = services_root.join(format!(".offloadkit-finder-staging-{transaction_id}"));
    let backup = services_root.join(format!(".offloadkit-finder-backup-{transaction_id}"));

    if let Err(error) = stage_workflows(&staging, &workflows) {
        let _ = remove_path_if_exists(&staging);
        return Err(FinderIntegrationError::new(
            "workflowInstallFailed",
            format!("Cannot stage Finder Quick Actions: {error}"),
        ));
    }

    let mut installed = Vec::new();
    let mut backed_up = Vec::new();
    let transaction = (|| {
        for (index, workflow) in workflows.iter().enumerate() {
            let destination = services_root.join(workflow.bundle_name);
            if fs::symlink_metadata(&destination).is_ok() {
                fs::create_dir_all(&backup)?;
                let backup_path = backup.join(workflow.bundle_name);
                fs::rename(&destination, &backup_path)?;
                backed_up.push(workflow.bundle_name);
            }
            if fail_after == Some(index) {
                return Err(io::Error::other(
                    "simulated Finder workflow install failure",
                ));
            }
            fs::rename(staging.join(workflow.bundle_name), &destination)?;
            installed.push(workflow.bundle_name);
        }
        let status = status_at(services_root, executable, require_production_executable)
            .map_err(|error| io::Error::other(error.message))?;
        if !status.healthy {
            return Err(io::Error::other(status.message.unwrap_or_else(|| {
                "Finder workflow read-back was unhealthy".to_owned()
            })));
        }
        Ok(status)
    })();

    match transaction {
        Ok(status) => {
            remove_path_if_exists(&staging).map_err(|error| {
                FinderIntegrationError::new(
                    "workflowInstallFailed",
                    format!("Installed workflows but cannot remove staging: {error}"),
                )
            })?;
            remove_path_if_exists(&backup).map_err(|error| {
                FinderIntegrationError::new(
                    "workflowInstallFailed",
                    format!("Installed workflows but cannot remove backup: {error}"),
                )
            })?;
            Ok(status)
        }
        Err(error) => {
            let rollback =
                rollback_install(services_root, &staging, &backup, &installed, &backed_up);
            let rollback_message = match rollback {
                Ok(()) => "previous workflows were restored".to_owned(),
                Err(rollback_error) => format!("rollback also failed: {rollback_error}"),
            };
            Err(FinderIntegrationError::new(
                "workflowInstallFailed",
                format!("Cannot install Finder Quick Actions: {error}; {rollback_message}"),
            ))
        }
    }
}

fn ensure_services_root(services_root: &Path) -> Result<(), FinderIntegrationError> {
    if !services_root.is_absolute() {
        return Err(FinderIntegrationError::new(
            "invalidServicesPath",
            "Finder Services path must be absolute",
        ));
    }
    fs::create_dir_all(services_root).map_err(|error| {
        FinderIntegrationError::new(
            "servicesCreateFailed",
            format!(
                "Cannot create Finder Services directory {}: {error}",
                services_root.display()
            ),
        )
    })?;
    reject_symlink_components(services_root).map_err(|error| {
        FinderIntegrationError::new(
            "unsafeServicesPath",
            format!("Finder Services path is unsafe: {error}"),
        )
    })?;
    let metadata = fs::symlink_metadata(services_root).map_err(|error| {
        FinderIntegrationError::new(
            "servicesReadFailed",
            format!("Cannot inspect Finder Services directory: {error}"),
        )
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(FinderIntegrationError::new(
            "unsafeServicesPath",
            "Finder Services path must be a real directory",
        ));
    }
    Ok(())
}

fn reject_symlink_components(path: &Path) -> io::Result<()> {
    for component in path.ancestors() {
        match fs::symlink_metadata(component) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("filesystem link is not allowed: {}", component.display()),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn stage_workflows(staging: &Path, workflows: &[RenderedWorkflow]) -> io::Result<()> {
    fs::create_dir(staging)?;
    for workflow in workflows {
        let contents = staging.join(workflow.bundle_name).join("Contents");
        fs::create_dir_all(&contents)?;
        write_synced(&contents.join("Info.plist"), &workflow.info_plist)?;
        write_synced(&contents.join("document.wflow"), &workflow.document_wflow)?;
        if !workflow_matches(&staging.join(workflow.bundle_name), workflow)? {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("staged workflow failed read-back: {}", workflow.bundle_name),
            ));
        }
    }
    Ok(())
}

fn write_synced(path: &Path, contents: &str) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()
}

fn workflow_matches(bundle: &Path, expected: &RenderedWorkflow) -> io::Result<bool> {
    let metadata = match fs::symlink_metadata(bundle) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Ok(false);
    }
    let contents = bundle.join("Contents");
    let contents_metadata = match fs::symlink_metadata(&contents) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if !contents_metadata.is_dir() || contents_metadata.file_type().is_symlink() {
        return Ok(false);
    }
    file_matches(&contents.join("Info.plist"), expected.info_plist.as_bytes()).and_then(
        |info_matches| {
            if !info_matches {
                return Ok(false);
            }
            file_matches(
                &contents.join("document.wflow"),
                expected.document_wflow.as_bytes(),
            )
        },
    )
}

fn file_matches(path: &Path, expected: &[u8]) -> io::Result<bool> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Ok(false);
    }
    Ok(fs::read(path)? == expected)
}

fn rollback_install(
    services_root: &Path,
    staging: &Path,
    backup: &Path,
    installed: &[&str],
    backed_up: &[&str],
) -> io::Result<()> {
    let mut errors = Vec::new();
    for bundle_name in installed.iter().rev() {
        if let Err(error) = remove_path_if_exists(&services_root.join(bundle_name)) {
            errors.push(format!("remove {bundle_name}: {error}"));
        }
    }
    for bundle_name in backed_up.iter().rev() {
        let backup_path = backup.join(bundle_name);
        if fs::symlink_metadata(&backup_path).is_ok() {
            if let Err(error) = fs::rename(&backup_path, services_root.join(bundle_name)) {
                errors.push(format!("restore {bundle_name}: {error}"));
            }
        }
    }
    for path in [staging, backup] {
        if let Err(error) = remove_path_if_exists(path) {
            errors.push(format!("remove {}: {error}", path.display()));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(io::Error::other(errors.join("; ")))
    }
}

fn remove_path_if_exists(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => remove_path(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn remove_path(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return fs::remove_file(path).or_else(|_| fs::remove_dir(path));
    }
    if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

#[cfg(target_os = "macos")]
fn services_root_for_current_user() -> Result<PathBuf, FinderIntegrationError> {
    let home = std::env::var_os("HOME").ok_or_else(|| {
        FinderIntegrationError::new(
            "homeDirectoryUnavailable",
            "Cannot resolve HOME for the current macOS user",
        )
    })?;
    let home = PathBuf::from(home);
    if !home.is_absolute() {
        return Err(FinderIntegrationError::new(
            "homeDirectoryUnavailable",
            "The current macOS HOME path is not absolute",
        ));
    }
    Ok(home.join("Library/Services"))
}

#[cfg(target_os = "macos")]
fn current_executable() -> Result<PathBuf, FinderIntegrationError> {
    std::env::current_exe().map_err(|error| {
        FinderIntegrationError::new(
            "currentExecutableUnavailable",
            format!("Cannot resolve the OffloadKit executable: {error}"),
        )
    })
}

#[cfg(target_os = "macos")]
pub fn finder_integration_status_for_current_user(
) -> Result<FinderIntegrationStatus, FinderIntegrationError> {
    status_at(
        &services_root_for_current_user()?,
        &current_executable()?,
        true,
    )
}

#[cfg(not(target_os = "macos"))]
pub fn finder_integration_status_for_current_user(
) -> Result<FinderIntegrationStatus, FinderIntegrationError> {
    Ok(unsupported_status())
}

#[cfg(target_os = "macos")]
pub fn install_finder_integration_for_current_user(
) -> Result<FinderIntegrationStatus, FinderIntegrationError> {
    let mut status = install_at(
        &services_root_for_current_user()?,
        &current_executable()?,
        true,
    )?;
    append_refresh_guidance(&mut status, refresh_services_cache());
    Ok(status)
}

#[cfg(not(target_os = "macos"))]
pub fn install_finder_integration_for_current_user(
) -> Result<FinderIntegrationStatus, FinderIntegrationError> {
    Err(FinderIntegrationError::new(
        "unsupportedPlatform",
        "Finder Quick Actions are only available on macOS",
    ))
}

#[cfg(target_os = "macos")]
pub fn uninstall_finder_integration_for_current_user(
) -> Result<FinderIntegrationStatus, FinderIntegrationError> {
    let executable = current_executable()?;
    let mut status = uninstall_at(&services_root_for_current_user()?, &executable, true)?;
    append_refresh_guidance(&mut status, refresh_services_cache());
    Ok(status)
}

#[cfg(not(target_os = "macos"))]
pub fn uninstall_finder_integration_for_current_user(
) -> Result<FinderIntegrationStatus, FinderIntegrationError> {
    Err(FinderIntegrationError::new(
        "unsupportedPlatform",
        "Finder Quick Actions are only available on macOS",
    ))
}

#[cfg(not(target_os = "macos"))]
fn unsupported_status() -> FinderIntegrationStatus {
    FinderIntegrationStatus {
        supported: false,
        installed: false,
        healthy: false,
        misplaced_app: false,
        executable_path: String::new(),
        expected_workflows: 0,
        installed_workflows: 0,
        matching_workflows: 0,
        problems: vec!["Finder Quick Actions are only available on macOS".to_owned()],
        message: Some("Finder Quick Actions are only available on macOS".to_owned()),
    }
}

#[cfg(target_os = "macos")]
fn refresh_services_cache() -> Option<String> {
    let tool = Path::new("/System/Library/CoreServices/pbs");
    let refreshed = tool.is_file()
        && std::process::Command::new(tool)
            .arg("-flush")
            .status()
            .is_ok_and(|status| status.success());
    (!refreshed).then(|| {
        "Quick Actions are installed, but macOS did not refresh the Services cache. Relaunch Finder or log out and back in if the menu does not update.".to_owned()
    })
}

#[cfg(target_os = "macos")]
fn append_refresh_guidance(status: &mut FinderIntegrationStatus, guidance: Option<String>) {
    if let Some(guidance) = guidance {
        status.message = Some(match status.message.take() {
            Some(message) => format!("{message}; {guidance}"),
            None => guidance,
        });
    }
}

#[tauri::command]
pub fn install_finder_integration() -> Result<FinderIntegrationStatus, FinderIntegrationError> {
    install_finder_integration_for_current_user()
}

#[tauri::command]
pub fn uninstall_finder_integration() -> Result<FinderIntegrationStatus, FinderIntegrationError> {
    uninstall_finder_integration_for_current_user()
}

#[tauri::command]
pub fn finder_integration_status() -> Result<FinderIntegrationStatus, FinderIntegrationError> {
    finder_integration_status_for_current_user()
}

#[derive(Clone, Copy)]
struct WorkflowDefinition {
    bundle_name: &'static str,
    bundle_identifier: &'static str,
    menu_label: &'static str,
    action: ExplorerAction,
    action_argument: &'static str,
    folder_only: bool,
    action_uuid: &'static str,
    input_uuid: &'static str,
    output_uuid: &'static str,
}

const WORKFLOW_DEFINITIONS: [WorkflowDefinition; 4] = [
    WorkflowDefinition {
        bundle_name: "OffloadKit Set Source.workflow",
        bundle_identifier: "com.offloadkit.finder.set-source",
        menu_label: "Đặt làm Source trong OffloadKit",
        action: ExplorerAction::SetSource,
        action_argument: "set-source",
        folder_only: false,
        action_uuid: "EACB97D4-C536-4E6D-B1DE-3C01AE8C98B1",
        input_uuid: "1F7849FC-B924-48E6-91AB-0808AC6FBE1D",
        output_uuid: "63065370-356D-47B3-AFAB-A349150B58DD",
    },
    WorkflowDefinition {
        bundle_name: "OffloadKit Set Destination.workflow",
        bundle_identifier: "com.offloadkit.finder.set-destination",
        menu_label: "Đặt làm Destination trong OffloadKit",
        action: ExplorerAction::SetDestination,
        action_argument: "set-destination",
        folder_only: true,
        action_uuid: "9F875275-52C3-45C5-9CDE-C9ED970745CE",
        input_uuid: "72367B26-4D9A-44F9-860C-64E596F8A1D9",
        output_uuid: "BA9BE9C2-A036-446B-A7E3-65F1BE4471BE",
    },
    WorkflowDefinition {
        bundle_name: "OffloadKit Copy.workflow",
        bundle_identifier: "com.offloadkit.finder.copy",
        menu_label: "Copy bằng OffloadKit",
        action: ExplorerAction::Copy,
        action_argument: "copy",
        folder_only: false,
        action_uuid: "E888655B-0C95-41ED-AF45-C0804720D17D",
        input_uuid: "A1E83943-D978-4261-907C-F584CDFE2C9C",
        output_uuid: "5170D63C-ED17-4945-AB57-29747036626A",
    },
    WorkflowDefinition {
        bundle_name: "OffloadKit Paste.workflow",
        bundle_identifier: "com.offloadkit.finder.paste",
        menu_label: "Paste và bắt đầu transfer",
        action: ExplorerAction::Paste,
        action_argument: "paste",
        folder_only: true,
        action_uuid: "7C06FB63-BD38-4A48-A8EF-E836799B0D0B",
        input_uuid: "E30EB8D4-63CB-4E62-B94B-0906F5DCC059",
        output_uuid: "6D1CE6F3-89C8-48FE-A541-B141EF830633",
    },
];

pub fn render_workflows(executable: &Path) -> io::Result<Vec<RenderedWorkflow>> {
    let executable_text = executable.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Finder workflow executable path is not valid Unicode",
        )
    })?;
    if executable_text.contains('\0') || !executable_text.starts_with('/') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Finder workflow executable path must be an absolute path without NUL characters",
        ));
    }

    WORKFLOW_DEFINITIONS
        .iter()
        .map(|definition| {
            Ok(RenderedWorkflow {
                bundle_name: definition.bundle_name,
                action: definition.action,
                executable_path: executable.to_path_buf(),
                info_plist: render_info_plist(definition),
                document_wflow: render_document_wflow(definition, executable_text),
            })
        })
        .collect()
}

fn render_info_plist(definition: &WorkflowDefinition) -> String {
    let input_uti = if definition.folder_only {
        "public.folder"
    } else {
        "public.item"
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>vi</string>
  <key>CFBundleIdentifier</key><string>{bundle_identifier}</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleName</key><string>{bundle_name}</string>
  <key>CFBundlePackageType</key><string>BNDL</string>
  <key>CFBundleShortVersionString</key><string>1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>NSServices</key>
  <array>
    <dict>
      <key>NSMenuItem</key><dict><key>default</key><string>{menu_label}</string></dict>
      <key>NSMessage</key><string>runWorkflowAsService</string>
      <key>NSRequiredContext</key><dict><key>NSApplicationIdentifier</key><string>com.apple.finder</string></dict>
      <key>NSSendFileTypes</key><array><string>{input_uti}</string></array>
      <key>NSUserData</key><string>{bundle_name}</string>
    </dict>
  </array>
</dict>
</plist>
"#,
        bundle_identifier = definition.bundle_identifier,
        bundle_name = xml_escape(definition.bundle_name.trim_end_matches(".workflow")),
        menu_label = xml_escape(definition.menu_label),
        input_uti = input_uti,
    )
}

fn render_document_wflow(definition: &WorkflowDefinition, executable: &str) -> String {
    let input_identifier = if definition.folder_only {
        "com.apple.Automator.folder"
    } else {
        "com.apple.Automator.fileSystemObject"
    };
    let executable = shell_single_quote(executable);
    let script = format!(
        "executable={executable}\nargs=(--finder-action {action})\nfor path in \"$@\"; do\n  args+=(--path \"$path\")\ndone\nexec \"$executable\" \"${{args[@]}}\"",
        action = definition.action_argument,
    );
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>AMApplicationBuild</key><string>523</string>
  <key>AMApplicationVersion</key><string>2.10</string>
  <key>AMDocumentVersion</key><string>2</string>
  <key>actions</key>
  <array>
    <dict>
      <key>action</key>
      <dict>
        <key>AMAccepts</key><dict><key>Container</key><string>List</string><key>Optional</key><false/><key>Types</key><array><string>com.apple.cocoa.path</string></array></dict>
        <key>AMActionVersion</key><string>2.0.3</string>
        <key>AMApplication</key><array><string>Automator</string></array>
        <key>AMProvides</key><dict><key>Container</key><string>List</string><key>Types</key><array><string>com.apple.cocoa.path</string></array></dict>
        <key>ActionBundlePath</key><string>/System/Library/Automator/Run Shell Script.action</string>
        <key>ActionName</key><string>Run Shell Script</string>
        <key>ActionParameters</key>
        <dict>
          <key>COMMAND_STRING</key><string>{script}</string>
          <key>CheckedForUserDefaultShell</key><true/>
          <key>inputMethod</key><integer>1</integer>
          <key>shell</key><string>/bin/bash</string>
          <key>source</key><string></string>
        </dict>
        <key>BundleIdentifier</key><string>com.apple.RunShellScript</string>
        <key>Class Name</key><string>RunShellScriptAction</string>
        <key>InputUUID</key><string>{input_uuid}</string>
        <key>OutputUUID</key><string>{output_uuid}</string>
        <key>UUID</key><string>{action_uuid}</string>
      </dict>
      <key>isViewVisible</key><false/>
    </dict>
  </array>
  <key>connectors</key><dict/>
  <key>workflowMetaData</key>
  <dict>
    <key>serviceApplicationBundleID</key><string>com.apple.finder</string>
    <key>serviceApplicationPath</key><string>/System/Library/CoreServices/Finder.app</string>
    <key>serviceInputTypeIdentifier</key><string>{input_identifier}</string>
    <key>serviceOutputTypeIdentifier</key><string>com.apple.Automator.nothing</string>
    <key>serviceProcessesInput</key><integer>0</integer>
    <key>workflowTypeIdentifier</key><string>com.apple.Automator.servicesMenu</string>
  </dict>
</dict>
</plist>
"#,
        input_uuid = definition.input_uuid,
        output_uuid = definition.output_uuid,
        action_uuid = definition.action_uuid,
        script = xml_escape(&script),
        input_identifier = input_identifier,
    )
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::{
        install_at, install_at_with_failure, render_workflows, status_at, uninstall_at,
        PRODUCTION_EXECUTABLE_PATH,
    };
    use crate::explorer_integration::ExplorerAction;
    use quick_xml::events::Event;
    use quick_xml::Reader;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn renders_four_named_workflow_bundles_with_fixed_actions_and_executable() {
        let rendered = render_workflows(Path::new(PRODUCTION_EXECUTABLE_PATH)).unwrap();
        let expected = [
            (
                "OffloadKit Set Source.workflow",
                ExplorerAction::SetSource,
                "set-source",
            ),
            (
                "OffloadKit Set Destination.workflow",
                ExplorerAction::SetDestination,
                "set-destination",
            ),
            ("OffloadKit Copy.workflow", ExplorerAction::Copy, "copy"),
            ("OffloadKit Paste.workflow", ExplorerAction::Paste, "paste"),
        ];

        assert_eq!(rendered.len(), expected.len());
        for (workflow, (bundle_name, action, action_arg)) in rendered.iter().zip(expected) {
            assert_eq!(workflow.bundle_name, bundle_name);
            assert_eq!(workflow.action, action);
            assert_eq!(
                workflow.executable_path,
                Path::new(PRODUCTION_EXECUTABLE_PATH)
            );
            assert!(workflow.document_wflow.contains(PRODUCTION_EXECUTABLE_PATH));
            assert!(workflow
                .document_wflow
                .contains(&format!("--finder-action {action_arg}")));
            assert!(workflow
                .document_wflow
                .contains("for path in &quot;$@&quot;"));
            assert!(workflow
                .document_wflow
                .contains("args+=(--path &quot;$path&quot;)"));
            assert!(workflow.document_wflow.contains("&quot;${args[@]}&quot;"));
            assert!(workflow
                .document_wflow
                .contains("<key>ActionParameters</key>"));
            let action_parameters = workflow
                .document_wflow
                .find("<key>ActionParameters</key>")
                .unwrap();
            let action_dict_close = workflow
                .document_wflow
                .find("      </dict>\n      <key>isViewVisible</key>")
                .unwrap();
            assert!(action_parameters < action_dict_close);
            assert!(!workflow.document_wflow.contains("<key>parameters</key>"));
        }
    }

    #[test]
    fn renderer_scopes_destination_and_paste_to_folders_and_emits_valid_plists() {
        let rendered = render_workflows(Path::new(PRODUCTION_EXECUTABLE_PATH)).unwrap();

        for workflow in rendered {
            assert_xml(&workflow.info_plist);
            assert_xml(&workflow.document_wflow);
            let folder_only = matches!(
                workflow.action,
                ExplorerAction::SetDestination | ExplorerAction::Paste
            );
            if folder_only {
                assert!(workflow
                    .info_plist
                    .contains("<string>public.folder</string>"));
                assert!(workflow
                    .document_wflow
                    .contains("<string>com.apple.Automator.folder</string>"));
            } else {
                assert!(workflow.info_plist.contains("<string>public.item</string>"));
                assert!(workflow
                    .document_wflow
                    .contains("<string>com.apple.Automator.fileSystemObject</string>"));
            }
            assert!(!workflow.document_wflow.contains("eval "));
            assert!(!workflow.document_wflow.contains("$*"));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn installed_workflow_runs_action_with_selected_folder_as_an_argument() {
        use std::os::unix::fs::PermissionsExt;
        use std::process::Command;

        let temp = tempdir().unwrap();
        let services = temp.path().join("Library/Services");
        let recorder = temp.path().join("OffloadKit Recorder");
        let recorded = temp.path().join("recorded-args");
        std::fs::write(
            &recorder,
            format!(
                "#!/bin/bash\nprintf '%s\\0' \"$@\" > {}\n",
                super::shell_single_quote(recorded.to_str().unwrap())
            ),
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&recorder).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&recorder, permissions).unwrap();
        install_at(&services, &recorder, false).unwrap();

        let selected_file = temp.path().join("Thẻ nhớ A 001.mov");
        let selected_folder = temp.path().join("Destination Folder");
        std::fs::write(&selected_file, b"video").unwrap();
        std::fs::create_dir(&selected_folder).unwrap();
        let cases = [
            (
                "OffloadKit Set Source.workflow",
                "set-source",
                &selected_file,
            ),
            (
                "OffloadKit Set Destination.workflow",
                "set-destination",
                &selected_folder,
            ),
            ("OffloadKit Copy.workflow", "copy", &selected_file),
            ("OffloadKit Paste.workflow", "paste", &selected_folder),
        ];

        for (bundle, action, selected) in cases {
            let output = Command::new("/usr/bin/automator")
                .arg("-i")
                .arg(selected)
                .arg(services.join(bundle))
                .output()
                .unwrap();

            assert!(
                output.status.success(),
                "automator failed for {bundle}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let args = std::fs::read(&recorded).unwrap();
            let expected = format!(
                "--finder-action\0{action}\0--path\0{}\0",
                selected.display()
            );
            assert_eq!(args, expected.as_bytes(), "wrong arguments for {bundle}");
        }
    }

    #[test]
    fn install_status_and_uninstall_manage_only_four_offloadkit_workflows() {
        let temp = tempdir().unwrap();
        let services = temp.path().join("Library/Services");
        let unrelated = services.join("Another App.workflow/Contents/document.wflow");
        std::fs::create_dir_all(unrelated.parent().unwrap()).unwrap();
        std::fs::write(&unrelated, "keep me").unwrap();
        let executable = Path::new(PRODUCTION_EXECUTABLE_PATH);

        let installed = install_at(&services, executable, false).unwrap();
        let status = status_at(&services, executable, false).unwrap();

        assert!(installed.installed && installed.healthy);
        assert_eq!(status.installed_workflows, 4);
        assert_eq!(status.matching_workflows, 4);
        for workflow in render_workflows(executable).unwrap() {
            assert!(services
                .join(workflow.bundle_name)
                .join("Contents/Info.plist")
                .is_file());
            assert!(services
                .join(workflow.bundle_name)
                .join("Contents/document.wflow")
                .is_file());
        }

        let first = uninstall_at(&services, executable, false).unwrap();
        let second = uninstall_at(&services, executable, false).unwrap();
        assert!(!first.installed && !second.installed);
        assert_eq!(std::fs::read_to_string(unrelated).unwrap(), "keep me");
    }

    #[test]
    fn status_detects_corruption_and_reinstall_repairs_the_executable_path() {
        let temp = tempdir().unwrap();
        let services = temp.path().join("Services");
        let old_executable =
            Path::new("/Applications/Old OffloadKit.app/Contents/MacOS/offloadkit");
        install_at(&services, old_executable, false).unwrap();
        let corrupt = services.join("OffloadKit Copy.workflow/Contents/document.wflow");
        std::fs::write(&corrupt, "corrupt").unwrap();

        let unhealthy = status_at(&services, old_executable, false).unwrap();
        assert!(unhealthy.installed);
        assert!(!unhealthy.healthy);
        assert_eq!(unhealthy.matching_workflows, 3);

        let repaired = install_at(&services, Path::new(PRODUCTION_EXECUTABLE_PATH), false).unwrap();
        assert!(repaired.healthy);
        let document = std::fs::read_to_string(corrupt).unwrap();
        assert!(document.contains(PRODUCTION_EXECUTABLE_PATH));
        assert!(!document.contains("Old OffloadKit.app"));
    }

    #[test]
    fn production_install_rejects_an_executable_outside_the_fixed_app_bundle() {
        let temp = tempdir().unwrap();
        let services = temp.path().join("Services");
        let misplaced =
            Path::new("/Users/operator/Downloads/OffloadKit.app/Contents/MacOS/offloadkit");

        let error = install_at(&services, misplaced, true).unwrap_err();
        let status = status_at(&services, misplaced, true).unwrap();

        assert_eq!(error.code, "misplacedApplication");
        assert!(error.message.contains("/Applications/OffloadKit.app"));
        assert!(status.misplaced_app);
        assert!(!status.healthy);
        assert!(status.message.unwrap().contains("Applications"));
        assert!(!services.exists());
    }

    #[test]
    fn failed_install_rolls_back_to_the_previous_healthy_workflows() {
        let temp = tempdir().unwrap();
        let services = temp.path().join("Services");
        let old_executable = Path::new("/Applications/Previous.app/Contents/MacOS/offloadkit");
        install_at(&services, old_executable, false).unwrap();

        let failure =
            install_at_with_failure(&services, Path::new(PRODUCTION_EXECUTABLE_PATH), false, 2)
                .unwrap_err();

        assert_eq!(failure.code, "workflowInstallFailed");
        let rolled_back = status_at(&services, old_executable, false).unwrap();
        assert!(rolled_back.healthy);
        assert_eq!(rolled_back.matching_workflows, 4);
    }

    fn assert_xml(xml: &str) {
        let mut reader = Reader::from_str(xml);
        let mut buffer = Vec::new();
        loop {
            match reader.read_event_into(&mut buffer).unwrap() {
                Event::Eof => break,
                _ => buffer.clear(),
            }
        }
    }
}
