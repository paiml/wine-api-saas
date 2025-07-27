use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use tower_http::cors::CorsLayer;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct Wine {
    id: i64,
    name: String,
    region: Option<String>,
    variety: Option<String>,
    rating: Option<f64>,
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WineFilters {
    region: Option<String>,
    variety: Option<String>,
    min_rating: Option<f64>,
    max_rating: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct VarietyInfo {
    count: i64,
    avg_rating: f64,
}

async fn get_wines(
    Query(filters): Query<WineFilters>,
    axum::extract::State(pool): axum::extract::State<SqlitePool>,
) -> Result<Json<Vec<Wine>>, StatusCode> {
    let base_query = "SELECT id, name, region, variety, rating, notes FROM wine_ratings";
    
    match (&filters.region, &filters.variety, filters.min_rating, filters.max_rating) {
        (None, None, None, None) => {
            let wines = sqlx::query_as::<_, Wine>(base_query)
                .fetch_all(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Json(wines))
        }
        _ => {
            let mut conditions = Vec::new();
            let mut query = base_query.to_string();
            
            if let Some(region) = &filters.region {
                conditions.push(format!("region LIKE '%{}%'", region.replace("'", "''")));
            }
            if let Some(variety) = &filters.variety {
                conditions.push(format!("variety LIKE '%{}%'", variety.replace("'", "''")));
            }
            if let Some(min_rating) = filters.min_rating {
                conditions.push(format!("rating >= {}", min_rating));
            }
            if let Some(max_rating) = filters.max_rating {
                conditions.push(format!("rating <= {}", max_rating));
            }
            
            if !conditions.is_empty() {
                query.push_str(" WHERE ");
                query.push_str(&conditions.join(" AND "));
            }
            
            let wines = sqlx::query_as::<_, Wine>(&query)
                .fetch_all(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            
            Ok(Json(wines))
        }
    }
}

async fn get_regions(
    axum::extract::State(pool): axum::extract::State<SqlitePool>,
) -> Result<Json<HashMap<String, i64>>, StatusCode> {
    let rows = sqlx::query("SELECT region, COUNT(*) as count FROM wine_ratings WHERE region IS NOT NULL GROUP BY region")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut regions = HashMap::new();
    for row in rows {
        let region: String = row.get("region");
        let count: i64 = row.get("count");
        regions.insert(region, count);
    }
    
    Ok(Json(regions))
}

async fn get_varieties(
    axum::extract::State(pool): axum::extract::State<SqlitePool>,
) -> Result<Json<HashMap<String, VarietyInfo>>, StatusCode> {
    let rows = sqlx::query("SELECT variety, COUNT(*) as count, AVG(rating) as avg_rating FROM wine_ratings WHERE variety IS NOT NULL AND rating IS NOT NULL GROUP BY variety")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut varieties = HashMap::new();
    for row in rows {
        let variety: String = row.get("variety");
        let count: i64 = row.get("count");
        let avg_rating: f64 = row.get("avg_rating");
        varieties.insert(variety, VarietyInfo { count, avg_rating });
    }
    
    Ok(Json(varieties))
}

async fn search_wines(
    Query(search): Query<SearchQuery>,
    axum::extract::State(pool): axum::extract::State<SqlitePool>,
) -> Result<Json<Vec<Wine>>, StatusCode> {
    let query = "SELECT id, name, region, variety, rating, notes FROM wine_ratings WHERE name LIKE ? OR notes LIKE ?";
    let search_term = format!("%{}%", search.q);
    
    let wines = sqlx::query_as::<_, Wine>(query)
        .bind(&search_term)
        .bind(&search_term)
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(wines))
}

async fn get_wines_by_region(
    Path(region): Path<String>,
    axum::extract::State(pool): axum::extract::State<SqlitePool>,
) -> Result<Json<Vec<Wine>>, StatusCode> {
    let wines = sqlx::query_as::<_, Wine>("SELECT id, name, region, variety, rating, notes FROM wine_ratings WHERE region = ?")
        .bind(region)
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(wines))
}

pub fn create_app(pool: SqlitePool) -> Router {
    Router::new()
        .route("/wines/search", get(search_wines))
        .route("/wines/region/:region", get(get_wines_by_region))
        .route("/wines", get(get_wines))
        .route("/regions", get(get_regions))
        .route("/varieties", get(get_varieties))
        .layer(CorsLayer::permissive())
        .with_state(pool)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:wine_ratings.db".to_string());
    let pool = SqlitePool::connect(&database_url).await?;
    let app = create_app(pool);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Wine API server running on http://0.0.0.0:3000");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        
        sqlx::query(
            "CREATE TABLE wine_ratings (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                region TEXT,
                variety TEXT,
                rating REAL,
                notes TEXT
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO wine_ratings (id, name, region, variety, rating, notes) VALUES 
            (1, 'Test Cabernet 2020', 'California', 'Red Wine', 92.5, 'Rich and bold with notes of cherry'),
            (2, 'Test Chardonnay 2021', 'California', 'White Wine', 88.0, 'Crisp and clean with citrus notes'),
            (3, 'Test Pinot Noir 2019', 'Oregon', 'Red Wine', 90.0, 'Light bodied with earthy undertones'),
            (4, 'Bourbon Barrel Aged Red', 'Texas', 'Red Wine', 95.0, 'Aged in bourbon barrels with vanilla notes'),
            (5, 'Test Sauvignon Blanc', 'Washington', 'White Wine', 86.5, 'Fresh and herbaceous')"
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_get_all_wines() {
        let pool = setup_test_db().await;
        let app = create_app(pool);
        let server = TestServer::new(app).unwrap();

        let response = server.get("/wines").await;
        response.assert_status_ok();
        
        let wines: Vec<Wine> = response.json();
        assert_eq!(wines.len(), 5);
    }

    #[tokio::test]
    async fn test_filter_wines_by_region() {
        let pool = setup_test_db().await;
        let app = create_app(pool);
        let server = TestServer::new(app).unwrap();

        let response = server.get("/wines").add_query_param("region", "California").await;
        response.assert_status_ok();
        
        let wines: Vec<Wine> = response.json();
        assert_eq!(wines.len(), 2);
        assert!(wines.iter().all(|w| w.region.as_ref().unwrap().contains("California")));
    }

    #[tokio::test]
    async fn test_filter_wines_by_rating() {
        let pool = setup_test_db().await;
        let app = create_app(pool);
        let server = TestServer::new(app).unwrap();

        let response = server.get("/wines").add_query_param("min_rating", "90").await;
        response.assert_status_ok();
        
        let wines: Vec<Wine> = response.json();
        assert_eq!(wines.len(), 3);
        assert!(wines.iter().all(|w| w.rating.unwrap() >= 90.0));
    }

    #[tokio::test]
    async fn test_get_regions() {
        let pool = setup_test_db().await;
        let app = create_app(pool);
        let server = TestServer::new(app).unwrap();

        let response = server.get("/regions").await;
        response.assert_status_ok();
        
        let regions: HashMap<String, i64> = response.json();
        assert_eq!(regions.len(), 4);
        assert_eq!(regions.get("California"), Some(&2));
        assert_eq!(regions.get("Oregon"), Some(&1));
    }

    #[tokio::test]
    async fn test_get_varieties() {
        let pool = setup_test_db().await;
        let app = create_app(pool);
        let server = TestServer::new(app).unwrap();

        let response = server.get("/varieties").await;
        response.assert_status_ok();
        
        let varieties: HashMap<String, VarietyInfo> = response.json();
        assert_eq!(varieties.len(), 2);
        
        let red_wine = varieties.get("Red Wine").unwrap();
        assert_eq!(red_wine.count, 3);
        assert!((red_wine.avg_rating - 92.5).abs() < 1.0);
    }

    #[tokio::test]
    async fn test_search_wines() {
        let pool = setup_test_db().await;
        let app = create_app(pool);
        let server = TestServer::new(app).unwrap();

        let response = server.get("/wines/search").add_query_param("q", "bourbon").await;
        response.assert_status_ok();
        
        let wines: Vec<Wine> = response.json();
        assert_eq!(wines.len(), 1);
        assert!(wines[0].name.contains("Bourbon") || wines[0].notes.as_ref().unwrap().contains("bourbon"));
    }

    #[tokio::test]
    async fn test_get_wines_by_region() {
        let pool = setup_test_db().await;
        let app = create_app(pool);
        let server = TestServer::new(app).unwrap();

        let response = server.get("/wines/region/California").await;
        response.assert_status_ok();
        
        let wines: Vec<Wine> = response.json();
        assert_eq!(wines.len(), 2);
        assert!(wines.iter().all(|w| w.region.as_ref().unwrap() == "California"));
    }

    #[tokio::test]
    async fn test_get_wines_by_nonexistent_region() {
        let pool = setup_test_db().await;
        let app = create_app(pool);
        let server = TestServer::new(app).unwrap();

        let response = server.get("/wines/region/NonExistent").await;
        response.assert_status_ok();
        
        let wines: Vec<Wine> = response.json();
        assert_eq!(wines.len(), 0);
    }
}
