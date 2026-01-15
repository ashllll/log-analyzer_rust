//! Frontend error reporting commands

use serde::{Deserialize, Serialize};
use tauri::command;
use tracing::{error, info};

/// Frontend error report structure
#[derive(Debug, Serialize, Deserialize)]
pub struct FrontendErrorReport {
    pub error: String,
    pub stack: Option<String>,
    pub timestamp: String,
    #[serde(rename = "userAgent")]
    pub user_agent: String,
    pub url: String,
    pub component: Option<String>,
    pub user_action: Option<String>,
}

/// Report a frontend error to the backend
#[command]
pub async fn report_frontend_error(report: FrontendErrorReport) -> Result<(), String> {
    // Log the error with structured logging
    error!(
        error = %report.error,
        stack = ?report.stack,
        timestamp = %report.timestamp,
        user_agent = %report.user_agent,
        url = %report.url,
        component = ?report.component,
        user_action = ?report.user_action,
        "Frontend error reported"
    );

    // Report to Sentry if configured
    sentry::with_scope(
        |scope| {
            scope.set_tag("error_source", "frontend");
            scope.set_tag(
                "component",
                report.component.as_deref().unwrap_or("unknown"),
            );
            scope.set_context(
                "error_details",
                sentry::protocol::Context::Other({
                    let mut map = sentry::protocol::Map::new();
                    map.insert("timestamp".to_string(), report.timestamp.into());
                    map.insert("user_agent".to_string(), report.user_agent.into());
                    map.insert("url".to_string(), report.url.into());
                    if let Some(user_action) = &report.user_action {
                        map.insert("user_action".to_string(), user_action.clone().into());
                    }
                    map
                }),
            );
        },
        || {
            sentry::capture_message(&report.error, sentry::Level::Error);
        },
    );

    info!("Frontend error report processed successfully");
    Ok(())
}

/// Submit user feedback
#[command]
pub async fn submit_user_feedback(feedback: UserFeedback) -> Result<(), String> {
    // Log the feedback
    info!(
        rating = feedback.rating,
        category = %feedback.category,
        description = %feedback.description,
        error_id = ?feedback.error_id,
        email = ?feedback.email,
        "User feedback submitted"
    );

    // Report to Sentry if configured
    let description_clone = feedback.description.clone();
    sentry::with_scope(
        |scope| {
            scope.set_tag("feedback_category", &feedback.category);
            scope.set_tag("feedback_rating", feedback.rating.to_string());
            if let Some(error_id) = &feedback.error_id {
                scope.set_tag("related_error_id", error_id);
            }
            scope.set_context(
                "feedback_details",
                sentry::protocol::Context::Other({
                    let mut map = sentry::protocol::Map::new();
                    map.insert(
                        "description".to_string(),
                        feedback.description.clone().into(),
                    );
                    if let Some(steps) = &feedback.reproduction_steps {
                        map.insert("reproduction_steps".to_string(), steps.clone().into());
                    }
                    if let Some(email) = &feedback.email {
                        map.insert("email".to_string(), email.clone().into());
                    }
                    map.insert("timestamp".to_string(), feedback.timestamp.clone().into());
                    map
                }),
            );
        },
        || {
            sentry::capture_message(
                &format!("User Feedback: {}", description_clone),
                sentry::Level::Info,
            );
        },
    );

    info!("User feedback processed successfully");
    Ok(())
}

/// Get error reporting statistics
#[command]
pub async fn get_error_statistics() -> Result<ErrorStatistics, String> {
    // This could be expanded to track error statistics
    Ok(ErrorStatistics {
        total_errors: 0,
        frontend_errors: 0,
        backend_errors: 0,
        last_error_timestamp: None,
    })
}

/// User feedback structure
#[derive(Debug, Serialize, Deserialize)]
pub struct UserFeedback {
    pub rating: u8,
    pub description: String,
    pub email: Option<String>,
    pub category: String,
    pub reproduction_steps: Option<String>,
    pub error_id: Option<String>,
    pub error_message: Option<String>,
    pub context: Option<serde_json::Value>,
    pub timestamp: String,
    pub user_agent: String,
    pub url: String,
}

/// Error statistics structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorStatistics {
    pub total_errors: u64,
    pub frontend_errors: u64,
    pub backend_errors: u64,
    pub last_error_timestamp: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_report_frontend_error() {
        let report = FrontendErrorReport {
            error: "Test error".to_string(),
            stack: Some("Test stack trace".to_string()),
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            user_agent: "Test User Agent".to_string(),
            url: "http://localhost:3000".to_string(),
            component: Some("TestComponent".to_string()),
            user_action: Some("button_click".to_string()),
        };

        let result = report_frontend_error(report).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_error_statistics() {
        let result = get_error_statistics().await;
        assert!(result.is_ok());

        let stats = result.unwrap();
        assert_eq!(stats.total_errors, 0);
        assert_eq!(stats.frontend_errors, 0);
        assert_eq!(stats.backend_errors, 0);
    }
}
