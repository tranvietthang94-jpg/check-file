use crate::explorer_integration::ExplorerAction;
use std::io;
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
        <key>BundleIdentifier</key><string>com.apple.RunShellScript</string>
        <key>Class Name</key><string>RunShellScriptAction</string>
        <key>InputUUID</key><string>{input_uuid}</string>
        <key>OutputUUID</key><string>{output_uuid}</string>
        <key>UUID</key><string>{action_uuid}</string>
      </dict>
      <key>isViewVisible</key><false/>
      <key>parameters</key>
      <dict>
        <key>COMMAND_STRING</key><string>{script}</string>
        <key>CheckedForUserDefaultShell</key><true/>
        <key>inputMethod</key><integer>1</integer>
        <key>shell</key><string>/bin/bash</string>
        <key>source</key><string></string>
      </dict>
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
    use super::{render_workflows, PRODUCTION_EXECUTABLE_PATH};
    use crate::explorer_integration::ExplorerAction;
    use quick_xml::events::Event;
    use quick_xml::Reader;
    use std::path::Path;

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
