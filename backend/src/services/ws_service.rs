use crate::AppState;
use tokio::sync::broadcast;
use uuid::Uuid;

pub struct WsService;

impl WsService {
    pub async fn get_or_create_channel(
        state: &AppState,
        event_id: Uuid,
    ) -> broadcast::Sender<String> {
        let mut channels = state.event_channels.lock().await;

        if let Some(sender) = channels.get(&event_id) {
            sender.clone()
        } else {
            let (sender, _receiver) = broadcast::channel(100);
            channels.insert(event_id, sender.clone());
            sender
        }
    }

    pub async fn broadcast_to_event(state: &AppState, event_id: Uuid, message: String) {
        let channels = state.event_channels.lock().await;
        if let Some(sender) = channels.get(&event_id) {
            let _ = sender.send(message);
        }
    }

    pub async fn broadcast_pulse(state: &AppState, event_id: Uuid) {
        if let Ok(pulse) =
            crate::repositories::event_repository::get_event_pulse(&state.pool, event_id).await
        {
            if let Ok(json) = serde_json::to_string(&pulse) {
                Self::broadcast_to_event(
                    state,
                    event_id,
                    format!("{{\"type\":\"PULSE\",\"data\":{}}}", json),
                )
                .await;
            }
        }
    }

    pub async fn broadcast_seats_updated(state: &AppState, event_id: Uuid) {
        Self::broadcast_to_event(state, event_id, "{\"type\":\"SEATS_UPDATED\"}".to_string()).await;
    }
}
