use futures::StreamExt;
use network_interface::NetworkInterface;
use network_interface::NetworkInterfaceConfig;
use sysinfo::{Disks, System};
use tauri::Manager;
use tauri::path::PathResolver;
use tokio;
use tauri_plugin_android_fs::{AndroidFs, AndroidFsExt, FileUri, InitialLocation, PersistableAccessMode, PrivateDir};

use actix_web::{App, HttpServer};
use actix_web_httpauth::{extractors::basic::BasicAuth, middleware::HttpAuthentication};
use actix_web::{
    dev::ServiceRequest, error::ErrorUnauthorized, Error as ActixError};

async fn do_auth(
    req: ServiceRequest,
    creds: BasicAuth,
) -> Result<ServiceRequest, (ActixError, ServiceRequest)> {
    if creds.user_id() == "user" && creds.password() == Some("2379612fc9111043a09140c9e080ed537e19a2789e99d52d6e18cb1353797ab1") {
        Ok(req)
    } else {
        Err((ErrorUnauthorized("not authorized"), req))
    }
}

async fn actix_main() {
    let cur_path = std::env::current_dir().unwrap();
    println!("The current directory is {}", cur_path.display());

    //let html_content = std::fs::read_to_string("/storage/emulated/0/books/index.html").unwrap();
    //println!("---------------------------The content of the file is {}", html_content);
    let paths = std::fs::read_dir("/storage/emulated/0/").unwrap();

    for path in paths {
        println!("------------------------------------Name: {}", path.unwrap().path().display())
    }

    HttpServer::new(|| App::new()
            .wrap(HttpAuthentication::basic(do_auth))
            // .service(actix_files::Files::new("/", "./")
            .service(actix_files::Files::new("/", "/storage/emulated/0/")
            // .service(actix_files::Files::new("/", "/tmp")
                    .show_files_listing().use_hidden_files()
            ))
            .bind(("0.0.0.0", 4804)).unwrap()
            .run()
            .await;
}

#[tauri::command]
fn toggle_server(app: tauri::AppHandle) -> Result<String, String> {
    // check if the actix_main() is already running, and toggle the status
    let handle = tauri::async_runtime::spawn(actix_main());

    return Ok("done".to_string());

}

#[tauri::command]
fn folder_picker_example(app: tauri::AppHandle) -> Result<String, String> {
    let api = app.android_fs();

    // pick folder to read and write
    api.acquire_manage_external_storage();
    return Ok("done".to_string());
    let selected_folder = api.show_manage_dir_dialog(
        None, // Initial location
    ).unwrap();

    if let Some(selected_dir_uri) = selected_folder {
        // for entry in api.read_dir(&selected_dir_uri).unwrap() {
        //     match entry {
        //         Entry::File { name, uri, last_modified, len, mime_type, .. } => {
        // return Ok(format!("File: {} - {:?} - {:?} - {}", name, uri, last_modified, len))
        // },
        // Entry::Dir { name, uri, last_modified, .. } => {
        // return Ok(format!("Dir: {} - {:?} - {:?}", name, uri, last_modified))
        // },
        // }
        // }
        println!("reading /storage/emulated/0/books/index.html");
        println!("Selected folder: {:?}", &selected_dir_uri);
        let res3 = std::fs::read_to_string("/storage/emulated/0/books/index.html").unwrap();
        println!("res3: {:?}", res3);

        let res1 = api.check_persisted_uri_permission(&selected_dir_uri, PersistableAccessMode::ReadAndWrite).unwrap();
        println!("res1 {:?}", res1);
        let res2 = api.take_persistable_uri_permission(&selected_dir_uri).unwrap();
        println!("res2 {:?}", res2);
        let persisted_uri_perms = api.get_all_persisted_uri_permissions();
        for permission in persisted_uri_perms {
            println!("Persisted URI: {:?}", permission.collect::<Vec<_>>());
        }
        // let file_path: tauri_plugin_fs::FilePath = selected_dir_uri.into();
        // let file_path = PathResolver::file_name(selected_dir_uri);
            for entry in api.read_dir(&selected_dir_uri).unwrap() {
                match entry {
                    tauri_plugin_android_fs::Entry::File { name, uri, last_modified, len, mime_type, .. } => {
                        println!("***file {:?}", (name, uri, last_modified, len, mime_type));
                    },
                    tauri_plugin_android_fs::Entry::Dir { name, uri, last_modified, .. } => {
                        println!("***dir {:?}", (name, uri, last_modified));

                    },
                }
            }
        return Ok(format!("Selected folder: {:?}", selected_dir_uri));
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
                // tauri::async_runtime::spawn(actix_main());
                Ok(())
            })
            .invoke_handler(tauri::generate_handler![
            greet,
            file_picker_example,
            folder_picker_example,
                toggle_server,
            collect_nic_info
        ])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
}

