#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .manage(commands::olm::OlmState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::tree::get_tree,
            commands::fs::read_file,
            commands::fs::write_file,
            commands::fs::create_folder,
            commands::fs::create_file,
            commands::fs::rename_file,
            commands::fs::delete_path,
            commands::olm::olm_upsert_concept,
            commands::olm::olm_list_concepts,
            commands::olm::olm_add_edge,
            commands::olm::olm_list_edges,
            commands::olm::olm_upsert_content_item,
            commands::olm::olm_map_content_concept,
            commands::olm::olm_ingest_event,
            commands::olm::olm_remove_content_concept_maps,
            commands::olm::olm_get_state,
            commands::olm::olm_get_content_metrics,
            commands::olm::olm_next_content_to_study,
            commands::olm::olm_get_explain,
            commands::olm::olm_next_to_study,
            commands::olm::olm_get_config,
            commands::olm::olm_set_config,
            commands::olm::olm_reset_state,
            commands::olm::olm_export_json,
            commands::olm::olm_import_json,
            commands::olm::olm_save_state,
            commands::olm::olm_load_state,
            commands::olm::olm_get_debug_ranking,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
