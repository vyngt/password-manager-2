use crate::domain::app::entities::ExtensionSession;
use crate::domain::shared::{SessionId, StorageError};
use crate::infrastructure::sqlite::app::entities::extension_session::Model;

use super::{fixed_key, string_to_ts, string_to_ts_opt, ts_to_string};

pub fn model_to_domain(model: Model) -> Result<ExtensionSession, StorageError> {
    Ok(ExtensionSession {
        session_id: SessionId::from_raw(model.session_id),
        browser: model.browser,
        profile_name: model.profile_name,
        session_key: fixed_key(&model.session_key, "extension_session.session_key")?,
        created_at: string_to_ts(&model.created_at)?,
        last_active_at: string_to_ts_opt(model.last_active_at.as_deref())?,
    })
}

pub fn domain_to_model(session: &ExtensionSession) -> Model {
    Model {
        session_id: session.session_id.as_str().to_owned(),
        browser: session.browser.clone(),
        profile_name: session.profile_name.clone(),
        session_key: session.session_key.to_vec(),
        created_at: ts_to_string(&session.created_at),
        last_active_at: session.last_active_at.as_ref().map(ts_to_string),
    }
}
