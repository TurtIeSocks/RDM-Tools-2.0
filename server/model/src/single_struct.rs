use super::*;

pub type SingleStruct<T = Precision> = Vec<point_struct::PointStruct<T>>;

impl ToPointArray for SingleStruct {
    fn to_point_array(self) -> point_array::PointArray {
        [self[0].lat, self[0].lon]
    }
}

impl ToSingleVec for SingleStruct {
    fn to_single_vec(self) -> single_vec::SingleVec {
        self.into_iter()
            .map(|point| point.to_point_array())
            .collect()
    }
}

impl ToMultiVec for SingleStruct {
    fn to_multi_vec(self) -> multi_vec::MultiVec {
        vec![self.to_single_vec()]
    }
}

impl ToPointStruct for SingleStruct {
    fn to_struct(self) -> point_struct::PointStruct {
        println!("`to_struct()` was called on a SingleVec and this was likely unintentional, did you mean to map over the values first?");
        point_struct::PointStruct {
            lat: self[0].lat,
            lon: self[0].lon,
        }
    }
}

impl ToSingleStruct for SingleStruct {
    fn to_single_struct(self) -> SingleStruct {
        self
    }
}

impl ToMultiStruct for SingleStruct {
    fn to_multi_struct(self) -> multi_struct::MultiStruct {
        vec![self]
    }
}

impl ToFeature for SingleStruct {
    fn to_feature(self, enum_type: Option<&Type>) -> Feature {
        Feature {
            geometry: Some(Geometry {
                bbox: None,
                foreign_members: None,
                value: if let Some(enum_type) = enum_type {
                    self.to_multi_vec().get_geojson_value(enum_type)
                } else {
                    self.to_multi_vec().polygon()
                },
            }),
            ..Default::default()
        }
    }
}

impl ToCollection for SingleStruct {
    fn to_collection(self, enum_type: Option<&Type>) -> FeatureCollection {
        FeatureCollection {
            bbox: None,
            features: vec![self.to_feature(enum_type)],
            foreign_members: None,
        }
    }
}

impl ToText for SingleStruct {
    fn to_text(self, sep_1: &str, sep_2: &str) -> String {
        self.into_iter()
            .map(|each| each.to_text(sep_1, sep_2))
            .collect()
    }
}

impl ToPoracle for SingleStruct {
    fn to_poracle(self) -> poracle::Poracle {
        poracle::Poracle {
            path: Some(self.to_single_vec()),
            ..Default::default()
        }
    }
}
