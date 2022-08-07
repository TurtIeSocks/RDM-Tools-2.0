use super::*;
use crate::models::{api::RouteGeneration, scanner::InstanceData};
use crate::queries::instance::*;

#[get("/api/instance/all")]
async fn all(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let instances = web::block(move || {
        let conn = pool.get()?;
        query_all_instances(&conn)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().json(instances))
}

#[post("/api/instance/area")]
async fn area(
    pool: web::Data<DbPool>,
    payload: web::Json<RouteGeneration>,
) -> Result<HttpResponse, Error> {
    let instance = web::block(move || {
        let conn = pool.get()?;

        query_instance_route(&conn, &payload.name)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let data: InstanceData =
        serde_json::from_str(instance.data.as_str()).expect("JSON was not well-formatted");

    Ok(HttpResponse::Ok().json(data.area))
}
