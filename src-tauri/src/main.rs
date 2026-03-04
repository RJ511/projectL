#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .manage(commands::OlmState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_tree,
            commands::read_file,
            commands::write_file,
            commands::create_folder,
            commands::create_file,
            commands::rename_file,
            commands::olm_upsert_concept,
            commands::olm_list_concepts,
            commands::olm_add_edge,
            commands::olm_list_edges,
            commands::olm_upsert_content_item,
            commands::olm_map_content_concept,
            commands::olm_ingest_event,
            commands::olm_get_state,
            commands::olm_get_explain,
            commands::olm_next_to_study,
            commands::olm_get_config,
            commands::olm_set_config,
            commands::olm_reset_state,
            commands::olm_export_json,
            commands::olm_import_json,
            commands::olm_save_state,
            commands::olm_load_state,
            commands::olm_get_debug_ranking,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
