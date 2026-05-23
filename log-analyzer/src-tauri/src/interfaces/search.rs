//! Search command interface adapters.
//!
//! Keep Tauri-specific command signatures here and delegate to the existing
//! search implementation until the application use case reaches event parity.

use tauri::{AppHandle, State};

use la_core::error::CommandError;
use la_core::models::{SearchFilters, SearchQuery};

use crate::models::AppState;

#[tauri::command]
pub async fn search_logs(
    app: AppHandle,
    query: String,
    #[allow(non_snake_case)] structuredQuery: Option<SearchQuery>,
    #[allow(non_snake_case)] workspaceId: Option<String>,
    #[allow(non_snake_case)] maxResults: Option<usize>,
    filters: Option<SearchFilters>,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    crate::commands::search::search_logs_impl(
        app,
        query,
        structuredQuery,
        workspaceId,
        maxResults,
        filters,
        state,
    )
    .await
}
