use futures::StreamExt;
use network_interface::NetworkInterface;
use network_interface::NetworkInterfaceConfig;
use sysinfo::{Disks, System};
use tauri::Manager;
use tauri_plugin_android_fs::{AndroidFs, AndroidFsExt, Entry};
use tokio;


#[tauri::command]
fn folder_picker_example(app: tauri::AppHandle) -> Result<String, String> {
    let api = app.android_fs();
    let api = app.android_fs();

    // pick folder to read and write
    let selected_folder = api.show_manage_dir_dialog(
        None, // Initial location
    ).unwrap();

    if let Some(dir_uri) = selected_folder {
        // for entry in api.read_dir(&dir_uri).unwrap() {
        //     match entry {
        //         Entry::File { name, uri, last_modified, len, mime_type, .. } => {
        // return Ok(format!("File: {} - {:?} - {:?} - {}", name, uri, last_modified, len))
        // },
        // Entry::Dir { name, uri, last_modified, .. } => {
        // return Ok(format!("Dir: {} - {:?} - {:?}", name, uri, last_modified))
        // },
        // }
        // }
        return Ok(format!("Selected folder: {:?}", dir_uri));
    }
    return Err("Folder picker canceled".to_string());
}
#[tauri::command]
fn file_picker_example(app: tauri::AppHandle) -> Result<String, String> {
    let api = app.android_fs();
    let mut file_type = "file".to_string(); // Use a `String` instead of a reference

    let mut selected_files = api
            .show_open_file_dialog(
                None,     // Initial location
                &["*/*"], // Target MIME types
                true,     // Allow multiple files
            )
            .unwrap();

    if selected_files.is_empty() {
        Err("File picker canceled".to_string())
    } else {
        if selected_files.len() == 1 {
            let mime_type = api
                    .get_mime_type(&selected_files.pop().unwrap())
                    .unwrap()
                    .unwrap();
            file_type = mime_type; // Assign the `String` value
        } else {
            for uri in selected_files {
                let mime_type = api.get_mime_type(&uri).unwrap().unwrap();
                file_type = mime_type; // Assign the `String` value
                let file_name = api.get_name(&uri).unwrap();

                // Handle file operations if needed
                let file_path: tauri_plugin_fs::FilePath = uri.into();
            }
        }
        Ok(format!("File type: {}", file_type)) // Use the `String` value
    }
}// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(app_handle: tauri::AppHandle, name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}
// use futures::executor::block_on;
#[tauri::command]
fn collect_nic_info() -> String {
    let network_interfaces = NetworkInterface::show().unwrap();
    let mut result: String = "".to_owned();
    for itf in network_interfaces.iter() {
        result.push_str(&format!("{:?}", itf));
    }
    let mut sys = System::new_all();
    sys.refresh_all();
    result.push_str(&format!("=> system:"));
    // RAM and swap information:
    result.push_str(&format!("total memory: {} bytes", sys.total_memory()));
    result.push_str(&format!("used memory : {} bytes", sys.used_memory()));
    result.push_str(&format!("total swap  : {} bytes", sys.total_swap()));
    result.push_str(&format!("used swap   : {} bytes", sys.used_swap()));

    // Display system information:
    result.push_str(&format!("System name:             {:?}", System::name()));
    result.push_str(&format!(
        "System kernel version:   {:?}",
        System::kernel_version()
    ));
    result.push_str(&format!(
        "System OS version:       {:?}",
        System::os_version()
    ));
    result.push_str(&format!(
        "System host name:        {:?}",
        System::host_name()
    ));

    // Number of CPUs:
    result.push_str(&format!("NB CPUs: {}", sys.cpus().len()));

    // Display processes ID, name na disk usage:
    for (pid, process) in sys.processes() {
        result.push_str(&format!(
            "[{pid}] {:?} {:?}",
            process.name(),
            process.disk_usage()
        ));
    }

    // We display all disks' information:
    result.push_str(&format!("=> disks:"));
    let disks = Disks::new_with_refreshed_list();
    for disk in &disks {
        result.push_str(&format!("{disk:?}"));
    }

    return result;
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_android_fs::init())
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_view::init())
            .plugin(tauri_plugin_dialog::init())
            .setup(|app| {
                // std::thread::spawn(move || block_on(tcc_main()));
                // tauri::async_runtime::spawn(miniserve_main());
                Ok(())
            })
            .invoke_handler(tauri::generate_handler![
            greet,
            file_picker_example,
            folder_picker_example,
            collect_nic_info
        ])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
}

