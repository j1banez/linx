use crate::error::AppError;
use crate::validate::BASE62;
use rand::Rng;
use sqlx::SqlitePool;

pub async fn insert_link(db: &SqlitePool, code: &str, url: &str) -> Result<(), AppError> {
    sqlx::query("INSERT INTO link (code, url) VALUES (?, ?)")
        .bind(code)
        .bind(url)
        .execute(db)
        .await
        .map_err(|err| match err {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                AppError::Conflict("code already exists".to_string())
            }
            _ => AppError::Internal,
        })?;

    Ok(())
}

pub async fn fetch_link_url(db: &SqlitePool, code: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>("SELECT url FROM link WHERE code = ?")
        .bind(code)
        .fetch_optional(db)
        .await
}

pub async fn fetch_link_stats(
    db: &SqlitePool,
    code: &str,
) -> Result<Option<(String, i64, i64, Option<i64>)>, sqlx::Error> {
    sqlx::query_as::<_, (String, i64, i64, Option<i64>)>(
        "SELECT url, clicks, created_at, last_accessed_at
         FROM link
         WHERE code = ?",
    )
    .bind(code)
    .fetch_optional(db)
    .await
}

pub async fn bump_link_stats(db: &SqlitePool, code: &str) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE link
         SET clicks = clicks + 1,
             last_accessed_at = unixepoch()
         WHERE code = ?",
    )
    .bind(code)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn generate_and_insert(
    db: &SqlitePool,
    url: &str,
    code_len: usize,
) -> Result<String, AppError> {
    // Retry multiple times in case of collisions
    for _ in 0..5 {
        let code = generate_code(code_len);
        match insert_link(db, &code, url).await {
            Ok(()) => return Ok(code),
            Err(AppError::Internal) => return Err(AppError::Internal),
            Err(_) => continue,
        }
    }

    Err(AppError::Conflict(
        "unable to generate unique code".to_string(),
    ))
}

pub fn generate_code(length: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut code = String::with_capacity(length);

    for _ in 0..length {
        let index = rng.gen_range(0..BASE62.len());
        code.push(BASE62[index] as char);
    }

    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_code_has_correct_length_and_charset() {
        for len in [1usize, 2, 6, 12, 32] {
            let code = generate_code(len);
            assert_eq!(code.len(), len);
            assert!(code.bytes().all(|b| BASE62.contains(&b)));
        }
    }
}
