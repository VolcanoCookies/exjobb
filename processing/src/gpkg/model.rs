use ::sqlx::FromRow;
use geo::Geometry;
use geozero::wkb;

#[derive(Debug, FromRow)]
pub(crate) struct RawRoadRow {
    pub geom: wkb::Decode<Geometry<f64>>,
    #[sqlx(rename = "Vagnummer_Huvudnummer_Vard")]
    pub main_number: i32,
    #[sqlx(rename = "Vagnummer_Undernummer")]
    pub sub_number: i32,
    #[sqlx(rename = "_length")]
    pub length: f64,
    #[sqlx(rename = "id")]
    pub unique_id: i32,
    #[sqlx(rename = "Hastighetsgrans_HogstaTillatnaHastighet_F")]
    pub speed_limit_f: Option<String>,
    #[sqlx(rename = "Hastighetsgrans_HogstaTillatnaHastighet_B")]
    pub speed_limit_b: Option<String>,
    #[sqlx(rename = "Vagtrafiknat_Vagtrafiknattyp")]
    pub road_type: Option<String>,
    #[sqlx(rename = "ForbjudenFardriktning_F")]
    pub forbidden_direction_f: Option<String>,
    #[sqlx(rename = "ForbjudenFardriktning_B")]
    pub forbidden_direction_b: Option<String>,
}
