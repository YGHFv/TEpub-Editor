// Tauri's `desktop` cfg alias is not consistently available during the
// Android Gradle/Cargo invocation, so use the target OS for APIs that are
// unavailable on mobile platforms.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    utils::config::Color,
    Manager, WindowEvent,
};
mod fs_utils;
mod search;
mod templates;
mod font;
mod ai;
mod epub;
mod mobile;
mod library;
mod app;
mod toolbox;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn restore_primary_window(app: &tauri::AppHandle) {
    for label in ["toolbox", "main"] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
            return;
        }
    }

    if let Ok(window) =
        tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("/".into()))
            .title("TEpub Editor")
            .inner_size(1200.0, 740.0)
            .min_inner_size(1200.0, 740.0)
            .center()
            .build()
    {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                let app_bg = Some(Color(238, 244, 248, 255));
                for window in app.webview_windows().values() {
                    let _ = window.set_background_color(app_bg);
                }
                let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
                let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show_item, &quit_item])?;
                let icon = app
                    .default_window_icon()
                    .cloned()
                    .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".into()))?;
                TrayIconBuilder::new()
                    .tooltip("TEpub-Editor")
                    .icon(icon)
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "show" => {
                            restore_primary_window(app);
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            restore_primary_window(tray.app_handle());
                        }
                    })
                    .build(app)?;
            }
            #[cfg(any(target_os = "android", target_os = "ios"))]
            let _ = app;
            Ok(())
        })
        .on_window_event(|window, event| {
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    if window.label() == "toolbox" {
                        api.prevent_close();
                        window.app_handle().exit(0);
                    } else if window.label() == "main" {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
            #[cfg(any(target_os = "android", target_os = "ios"))]
            let _ = (window, event);
        })
        .invoke_handler(tauri::generate_handler![
            app::read_text_file,
            app::save_text_file,
            app::read_binary_file,
            library::search_book_covers,
            library::download_cover_to_temp,
            toolbox::download_remote_image,
            library::save_history,
            library::get_history_list,
            library::calculate_md5,
            font::list_library_fonts,
            font::import_library_font,
            font::rename_library_font,
            font::delete_library_font,
            templates::list_style_templates,
            templates::read_style_template,
            templates::import_style_template,
            templates::save_style_template,
            templates::restore_builtin_style_template,
            templates::list_epub_template_repositories,
            templates::add_epub_template_repository,
            templates::sync_epub_template_repository,
            templates::install_remote_epub_template,
            epub::scan_chapters,
            search::advanced_search,
            search::advanced_replace,
            epub::export_epub,
            epub::extract_epub,
            epub::load_epub_file_meta,
            epub::read_epub_file_content,
            epub::read_epub_file_binary,
            epub::get_epub_temp_dir_path,
            epub::read_epub_files_batch,
            epub::read_epub_binary_batch,
            font::analyze_epub_font_glyphs,
            epub::save_epub_file_content,
            epub::save_epub_to_disk,
            search::search_in_files,
            epub::add_epub_file,
            epub::add_epub_file_binary,
            epub::save_epub_file_binary,
            epub::save_epub_files_batch,
            epub::delete_epub_file,
            epub::rename_epub_file,
            app::get_launch_args, // Register new command
            app::get_launch_info,
            app::set_file_assoc,
            app::reveal_in_explorer,
            library::rename_book_file,
            library::rebuild_book_filenames,
            epub::prepare_epub_for_open,
            toolbox::toolbox_file_encrypt,
            toolbox::toolbox_file_decrypt,
            toolbox::toolbox_font_encrypt,
            toolbox::toolbox_font_decrypt,
            toolbox::toolbox_epub_reformat,
            toolbox::toolbox_image_convert,
            toolbox::toolbox_generate_ai_image,
            toolbox::toolbox_epub_diagnose,
            toolbox::toolbox_scan_batch_inputs,
            toolbox::toolbox_cancel_batch,
            toolbox::toolbox_run_batch,
            app::exit_app,
            // ===== Library Phase 1 =====
            library::load_library,
            library::save_library,
            ai::run_ai_proofing,
            ai::save_ai_proofing_log,
            ai::list_ai_proofing_logs,
            ai::read_ai_proofing_log,
            app::get_app_mode_info,
            // ===== Library Phase 2 =====
            library::add_book_to_library,
            library::remove_book_from_library,
            // ===== Library Phase 3 =====
            library::get_library_cover_data,
            library::refresh_book_metadata,
            mobile::mobile_cache_input_file,
            mobile::mobile_scan_chapters,
            mobile::mobile_make_epub,
            mobile::mobile_read_epub_metadata,
            mobile::mobile_read_epub_cover,
            mobile::mobile_update_epub_cover,
            mobile::mobile_update_epub_metadata,
            mobile::mobile_export_epub,
            // ===== Library Phase 4 =====
            library::update_book_metadata,
            library::update_book_cover
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
