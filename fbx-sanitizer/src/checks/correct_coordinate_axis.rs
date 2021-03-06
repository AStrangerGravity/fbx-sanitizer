use crate::utils::{get_application_name, ApplicationName};
use anyhow::format_err;
use cgmath::Vector3;
use fbxcel_dom::fbxcel::low::v7400::AttributeValue;
use fbxcel_dom::v7400::document::GlobalSettings;
use fbxcel_dom::v7400::Document;

/// This is a check for blender files generated by Blender 2.90. Unity does not currently support
/// fbx files generated like this.
/// https://forum.unity.com/threads/bake-axis-conversion-import-setting.899072/#post-6975023

/// In Blender 2.90, it is possible to export a file with the correct rotation, without changing the
/// axis. This guarantees that the object will not accidentally be counter-rotated when importing into Unity.
#[allow(unused)]
pub fn verify(doc: &Document) -> Result<Vec<String>, anyhow::Error> {
    let axis =
        get_coordinate_axis(doc).ok_or_else(|| format_err!("Could not find coordinate axis."))?;

    let application_name = get_application_name(doc);

    let correct = coordinate_axis_for_software(&application_name);

    if axis != correct {
        return Ok(vec![format!(
            "File has incorrect Coordinate Axis. Expected [{}] actual [{}]. [{:?}]",
            correct.display_triplet(),
            axis.display_triplet(),
            application_name,
        )]);
    }

    Ok(vec![])
}

#[derive(Debug, PartialEq, Eq)]
struct CoordinateAxis {
    up: Vector3<i8>,
    front: Vector3<i8>,
    coord: Vector3<i8>,
}

impl CoordinateAxis {
    fn display_triplet(&self) -> String {
        fn axis_letter(v: &Vector3<i8>) -> String {
            match v {
                Vector3 { x: 1, y: 0, z: 0 } => "+X".to_owned(),
                Vector3 { x: -1, y: 0, z: 0 } => "-X".to_owned(),
                Vector3 { x: 0, y: 1, z: 0 } => "+Y".to_owned(),
                Vector3 { x: 0, y: -1, z: 0 } => "-Y".to_owned(),
                Vector3 { x: 0, y: 0, z: 1 } => "+Z".to_owned(),
                Vector3 { x: 0, y: 0, z: -1 } => "-Z".to_owned(),
                // This should only hit if there is an error in this application.
                _ => panic!("Invalid Coordinate System"),
            }
        }

        let front = axis_letter(&self.front);
        let up = axis_letter(&self.up);
        let coord = axis_letter(&self.coord);

        format!("Front:{},Up:{},Coord:{}", front, up, coord)
    }
}

fn coordinate_axis_for_software(application_name: &Option<ApplicationName>) -> CoordinateAxis {
    match application_name {
        // 3DS Max should output in its native Z-up coordinate system. Then we check "Bake Coordinate Axis"
        // when importing.
        Some(ApplicationName::Max) => CoordinateAxis {
            up: Vector3 { x: 0, y: 0, z: 1 },
            front: Vector3 { x: 0, y: -1, z: 0 },
            coord: Vector3 { x: 1, y: 0, z: 0 },
        },

        Some(ApplicationName::Blender) =>
        // For Blender we export with a 180 flip from Blender's normal coordinates to fix:
        // https://forum.unity.com/threads/bake-axis-conversion-import-setting.899072/#post-6975023
        {
            CoordinateAxis {
                up: Vector3 { x: 0, y: 0, z: 1 },
                front: Vector3 { x: 0, y: 1, z: 0 },
                coord: Vector3 { x: -1, y: 0, z: 0 },
            }
        }

        _ =>
        // All other programs (ie. Maya) should output a coordinate system equivalent to Unity.
        {
            CoordinateAxis {
                up: Vector3 { x: 0, y: 1, z: 0 },
                front: Vector3 { x: 0, y: 0, z: 1 },
                coord: Vector3 { x: 1, y: 0, z: 0 },
            }
        }
    }
}

fn get_coordinate_axis(doc: &Document) -> Option<CoordinateAxis> {
    let global_settings = doc.global_settings()?;

    let up_axis = get_axis(&global_settings, "UpAxis")?;
    let front_axis = get_axis(&global_settings, "FrontAxis")?;
    let coord_axis = get_axis(&global_settings, "CoordAxis")?;

    Some(CoordinateAxis {
        up: up_axis,
        front: front_axis,
        coord: coord_axis,
    })
}

fn get_axis(global_settings: &GlobalSettings, name: &str) -> Option<Vector3<i8>> {
    let axis = if let AttributeValue::I32(v) = global_settings
        .raw_properties()
        .get_property(name)?
        .value_part()
        .get(0)?
    {
        v
    } else {
        return None;
    };

    let sign = if let AttributeValue::I32(v) = global_settings
        .raw_properties()
        .get_property(&(name.to_owned() + "Sign"))?
        .value_part()
        .get(0)?
    {
        v
    } else {
        return None;
    };

    Some(match axis {
        0 => [*sign as i8, 0, 0].into(),
        1 => [0, *sign as i8, 0].into(),
        2 => [0, 0, *sign as i8].into(),
        _ => return None,
    })
}
