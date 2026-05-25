use crate::presentation::response::error::AppError;
use validator::Validate;

pub fn validate_request<T: Validate>(req: &T) -> Result<(), AppError> {
    req.validate().map_err(|e| {
        let messages: Vec<String> = e
            .field_errors()
            .values()
            .flat_map(|errors| {
                errors
                    .iter()
                    .filter_map(|err| err.message.as_ref().map(|m| m.to_string()))
            })
            .collect();
        AppError::Validation(messages)
    })
}
