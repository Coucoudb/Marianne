use crate::profile::UserProfile;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_profile(state: State<'_, AppState>) -> Result<UserProfile, String> {
    Ok(state.profile.lock().clone())
}

#[tauri::command]
pub async fn save_profile(
    state: State<'_, AppState>,
    profile: UserProfile,
) -> Result<(), String> {
    profile.save(&state.data_dir).map_err(|e| e.to_string())?;
    *state.profile.lock() = profile;
    Ok(())
}
