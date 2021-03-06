use crate::ops::prelude::*;
use ndarray::*;

#[derive(Debug, Clone, new, Default)]
pub struct Slice {
    prune: Vec<(usize, usize)>,
}

impl Slice {
    fn eval_t<T: Datum>(&self, input: SharedTensor) -> TractResult<SharedTensor> {
        let input = input.to_array_view::<T>()?;
        let slice_spec: Vec<SliceOrIndex> = self
            .prune
            .iter()
            .map(|&(a, b)| SliceOrIndex::Slice {
                start: a as isize,
                end: if b != 0 { Some(-(b as isize)) } else { None },
                step: 1,
            })
            .collect();
        let slice_info = SliceInfo::<_, IxDyn>::new(slice_spec).unwrap();
        let slice = input.slice(&slice_info.as_ref());
        Ok(slice.to_owned().into())
    }
}

impl Op for Slice {
    fn name(&self) -> Cow<str> {
        "Slice".into()
    }

    fn pulsify(&self, mut inputs: TVec<&PulsedTensorFact>) -> TractResult<Vec<PulsifiedOp>> {
        let input = args_1!(inputs);
        if self
            .prune
            .iter()
            .enumerate()
            .all(|(ax, &(a, b))| ax == input.axis || (a == 0 && b == 0))
        {
            let delay = self.prune[input.axis].0;
            let mut fact = input.clone();
            fact.delay += delay;
            fact.dim -= delay.to_dim();
            return Ok(vec![PulsifiedOp::new(
                Box::new(crate::ops::identity::Identity::default()),
                tvec!(fact),
            )]);
        }
        unimplemented!();
    }
}

impl StatelessOp for Slice {
    /// Evaluates the operation given the input tensors.
    fn eval(&self, mut inputs: TVec<SharedTensor>) -> TractResult<TVec<SharedTensor>> {
        let input = args_1!(inputs);
        Ok(tvec!(dispatch_datum!(Self::eval_t(input.datum_type())(
            self, input
        ))?))
    }
}

impl InferenceRulesOp for Slice {
    fn rules<'r, 'p: 'r, 's: 'r>(
        &'s self,
        s: &mut Solver<'r>,
        inputs: &'p SharedTensorsProxy,
        outputs: &'p SharedTensorsProxy,
    ) -> InferenceResult {
        s.equals(&inputs.len, 1)?;
        s.equals(&outputs.len, 1)?;
        s.equals(&inputs[0].datum_type, &outputs[0].datum_type)?;
        s.equals(&inputs[0].rank, &outputs[0].rank)?;
        for (ix, &(a, b)) in self.prune.iter().enumerate() {
            s.equals(
                &inputs[0].shape[ix],
                outputs[0].shape[ix].bex() + a.to_dim() + b.to_dim(),
            )?;
        }
        Ok(())
    }
}
