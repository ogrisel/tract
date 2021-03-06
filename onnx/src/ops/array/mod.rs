mod slice;

use tract_core::ops as tractops;
use tract_core::ops::prelude::*;

use crate::ops::OpRegister;
use crate::pb;
use crate::pb::NodeProto;
use num_traits::AsPrimitive;

pub fn register_all_ops(reg: &mut OpRegister) {
    reg.insert("Concat", concat);
    reg.insert("ConstantLike", constant_like);
    reg.insert("Expand", |_| {
        Ok(Box::new(tractops::array::MultiBroadcastTo::default()))
    });
    reg.insert("EyeLike", eye_like);
    reg.insert("Flatten", flatten);
    reg.insert("Pad", pad);
    reg.insert("Reshape", |_| {
        Ok(Box::new(tractops::array::Reshape::default()))
    });
    reg.insert("Shape", |_| {
        Ok(Box::new(tractops::array::Shape::new(DatumType::I64)))
    });
    reg.insert("Size", |_| {
        Ok(Box::new(tractops::array::Size::new(DatumType::I64)))
    });
    reg.insert("Transpose", transpose);
    reg.insert("Slice", slice);
    reg.insert("Split", split);
    reg.insert("Squeeze", squeeze);
    reg.insert("Unsqueeze", unsqueeze);
}

pub fn concat(node: &NodeProto) -> TractResult<Box<Op>> {
    let axis = node.get_attr_int("axis")?;
    Ok(Box::new(tractops::array::Concat::new(axis as usize)))
}

pub fn make_const<T>(shape: &[usize], v: f32) -> TractResult<SharedTensor>
where
    T: Datum,
    f32: AsPrimitive<T>,
{
    Ok(::ndarray::Array::<T, _>::from_elem(shape, v.as_()).into())
}

pub fn constant_like(node: &NodeProto) -> TractResult<Box<Op>> {
    let value = node.get_attr_opt_float("value")?.unwrap_or(0.0);
    if node.get_input().len() == 0 {
        use protobuf::ProtobufEnum;
        let dt = match node.get_attr_opt_int("dtype")? {
            Some(dt) => pb::TensorProto_DataType::from_i32(dt as i32)
                .ok_or_else(|| {
                    format!("Can not convert integer {} into a TensorProto_DataType", dt)
                })?
                .tractify()?,
            None => f32::datum_type(),
        };
        let shape: Vec<usize> = node
            .get_attr_ints("shape")?
            .iter()
            .map(|&d| d as usize)
            .collect();
        let tensor = dispatch_numbers!(self::make_const(dt)(&shape, value))?;
        Ok(Box::new(tractops::konst::Const::new(tensor)))
    } else {
        Ok(Box::new(tractops::array::ConstantLike::new(value)))
    }
}

pub fn eye_like(node: &NodeProto) -> TractResult<Box<Op>> {
    use protobuf::ProtobufEnum;
    let dt = match node.get_attr_opt_int("dtype")? {
        Some(dt) => Some(
            pb::TensorProto_DataType::from_i32(dt as i32)
                .ok_or_else(|| {
                    format!("Can not convert integer {} into a TensorProto_DataType", dt)
                })?
                .tractify()?,
        ),
        None => None,
    };
    let k = node.get_attr_opt_int("k")?.unwrap_or(0);
    Ok(Box::new(tractops::array::EyeLike::new(dt, k as isize)))
}

pub fn flatten(node: &NodeProto) -> TractResult<Box<Op>> {
    let axis = node.get_attr_opt_int("axis")?.unwrap_or(1);
    Ok(Box::new(tractops::array::Flatten::new(axis as usize)))
}

pub fn pad(node: &NodeProto) -> TractResult<Box<Op>> {
    let mode = node.get_attr_opt_str("mode")?;
    let value = node.get_attr_opt_float("value")?;
    let mode = match mode {
        Some("reflect") => tractops::array::PadMode::Reflect,
        Some("edge") => tractops::array::PadMode::Edge,
        _ => tractops::array::PadMode::Constant(value.unwrap_or(0.0)),
    };
    let pads = node.get_attr_ints("pads")?;
    let rank = pads.len() / 2;
    let pads = (0..rank)
        .map(|ax| (pads[ax] as usize, pads[ax + rank] as usize))
        .collect();
    Ok(Box::new(tractops::array::Pad::new(pads, mode)))
}

pub fn slice(node: &NodeProto) -> TractResult<Box<Op>> {
    let axes = node.get_attr_opt_ints("axes")?;
    let begin = node.get_attr_ints("starts")?;
    let end = node.get_attr_ints("ends")?;
    Ok(Box::new(slice::Slice::new(
        axes.map(|a| a.into_iter().map(|&d| d as _).collect()),
        begin.iter().map(|&d| d as _).collect(),
        end.iter().map(|&d| d as _).collect(),
    )))
}

pub fn split(node: &NodeProto) -> TractResult<Box<Op>> {
    let axis = node.get_attr_opt_int("axis")?.unwrap_or(0);
    let split = node.get_attr_opt_ints("split")?;
    Ok(Box::new(tractops::array::Split::new(
        axis as usize,
        node.get_output().len(),
        split.map(|a| a.into_iter().map(|&d| d as _).collect()),
    )))
}

pub fn squeeze(node: &NodeProto) -> TractResult<Box<Op>> {
    let axes = node
        .get_attr_opt_ints("axes")?
        .map(|l| l.iter().map(|&a| a as usize).collect());
    Ok(Box::new(tractops::array::Squeeze::new(axes)))
}

pub fn transpose(node: &NodeProto) -> TractResult<Box<Op>> {
    let perm = node
        .get_attr_opt_ints("perm")?
        .map(|axes| axes.iter().map(|&a| a as usize).collect());
    Ok(Box::new(tractops::array::PermuteAxes::new(perm)))
}

pub fn unsqueeze(node: &NodeProto) -> TractResult<Box<Op>> {
    let axes = node
        .get_attr_ints("axes")?
        .iter()
        .map(|&a| a as usize)
        .collect();
    Ok(Box::new(tractops::array::AddDims::new(axes)))
}
