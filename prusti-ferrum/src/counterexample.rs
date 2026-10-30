//! Convert Ferrum's mapped counterexample to Prusti's serialized model.
use ferrum_silicon::reporting::{
    converter::{ExtractedFunction, ExtractedModel, ExtractedModelEntry, MappedModel},
    silicon_resolved_counterexample::{NativeCounterexample, NativeCounterexampleView},
};
use prusti_backend_types::{
    DomainEntry, Domains, FunctionEntry, Functions, Model, ModelEntry, SiliconCounterexample,
};
use rustc_hash::FxHashMap;
use std::sync::Arc;

fn entry(
    value: &ExtractedModelEntry,
    entries: &mut FxHashMap<String, ModelEntry>,
) -> Option<ModelEntry> {
    Some(match value {
        ExtractedModelEntry::Int(v) => ModelEntry::LitInt(v.to_string()),
        ExtractedModelEntry::Bool(v) => ModelEntry::LitBool(*v),
        ExtractedModelEntry::Perm(v) => ModelEntry::LitPerm(v.to_string()),
        ExtractedModelEntry::Ref { name, fields } => {
            let fields = fields
                .iter()
                .filter_map(|(field, (value, _))| entry(value, entries).map(|v| (field.clone(), v)))
                .collect();
            let result = ModelEntry::Ref(name.clone(), fields);
            entries.insert(name.clone(), result.clone());
            result
        }
        ExtractedModelEntry::NullRef(name) => ModelEntry::NullRef(name.clone()),
        ExtractedModelEntry::RecursiveRef(name) => ModelEntry::RecursiveRef(name.clone()),
        ExtractedModelEntry::Var { name, .. } => ModelEntry::Var(name.clone()),
        ExtractedModelEntry::Other { value, problem } => {
            ModelEntry::Other(value.clone(), problem.clone())
        }
        ExtractedModelEntry::Seq { name, values } => ModelEntry::Seq(
            name.clone(),
            values
                .iter()
                .filter_map(|value| entry(value, entries))
                .collect(),
        ),
        ExtractedModelEntry::DomainValue { domain, id } => {
            ModelEntry::DomainValue(domain.clone(), id.clone())
        }
        // The existing JVM bridge drops source model entry variants it does not recognize.
        _ => return None,
    })
}
fn model(source: &ExtractedModel) -> Model {
    let mut entries = FxHashMap::default();
    for (name, value) in &source.entries {
        if let Some(value) = entry(value, &mut entries) {
            entries.insert(name.clone(), value);
        }
    }
    Model { entries }
}
fn functions(
    source: &[ExtractedFunction],
    native: Option<&NativeCounterexample>,
) -> Result<Functions, String> {
    let mut entries = FxHashMap::default();
    for f in source {
        let mut scratch = FxHashMap::default();
        let default = entry(&f.default, &mut scratch);
        let mut options = Vec::new();
        for (args, result) in &f.options {
            let args = args
                .iter()
                .filter_map(|arg| entry(arg, &mut FxHashMap::default()).map(Some))
                .collect();
            let result = if let ExtractedModelEntry::Var { sort, .. } = result {
                use ferrum_silicon::state::heap::Heap;
                let native = native
                    .ok_or_else(|| format!("function {} result requires a native model", f.name))?;
                let heap = ferrum_silicon::reporting::converter::extract_heap_native(
                    native.heap.values(),
                    &native.native_model,
                )
                .map_err(|e| format!("counterexample heap: {e:?}"))?;
                let null_name = native
                    .native_model
                    .entries
                    .get("$Ref.null")
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "Ref!val!0".into());
                let resolved = ferrum_silicon::reporting::converter::map_local_var(
                    Some(sort),
                    result,
                    &heap,
                    &native.native_model,
                    &Default::default(),
                    &null_name,
                )
                .map_err(|e| format!("counterexample value: {e:?}"))?;
                entry(&resolved, &mut FxHashMap::default())
            } else {
                entry(result, &mut FxHashMap::default())
            };
            options.push((args, result));
        }
        options.reverse();
        entries.insert(f.name.clone(), FunctionEntry { options, default });
    }
    Ok(Functions { entries })
}
fn mapped(
    source: &MappedModel,
    native: &NativeCounterexample,
) -> Result<SiliconCounterexample, String> {
    let mut old_models = FxHashMap::default();
    let mut label_order = Vec::new();
    for (label, value) in &source.raw_old {
        label_order.push(label.clone());
        old_models.insert(label.clone(), model(value));
    }
    let mut domains = FxHashMap::default();
    for domain in &source.raw_domains {
        domains.insert(
            domain.name.clone(),
            DomainEntry {
                functions: functions(&domain.functions, Some(native))?,
            },
        );
    }
    Ok(SiliconCounterexample {
        model: model(&source.raw_current),
        functions: functions(&source.raw_functions, Some(native))?,
        domains: Domains { entries: domains },
        old_models,
        label_order,
    })
}
/// Return the last source failure context's mapped counterexample, if present.
pub fn from_diagnostic(
    error: &Arc<dyn ferrum_silver::verifier::verification_result::VerificationDiagnostic>,
) -> Result<Option<SiliconCounterexample>, String> {
    let contexts = error
        .failure_contexts()
        .map_err(|e| format!("failure contexts: {e:?}"))?;
    let Some(counterexample) = contexts.last().and_then(|c| c.counterexample()) else {
        return Ok(None);
    };
    let any: &dyn std::any::Any = counterexample.as_ref();
    let Some(counterexample) = any.downcast_ref::<NativeCounterexample>() else {
        return Ok(None);
    };
    match &counterexample.view {
        NativeCounterexampleView::Mapped(value) => mapped(value, counterexample).map(Some),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrum_silicon::state::terms::Sort;
    #[test]
    fn reference_entries_are_indexed_and_function_options_reversed() {
        let value = ExtractedModelEntry::Ref {
            name: "r".into(),
            fields: vec![("f".into(), (ExtractedModelEntry::Int(7.into()), None))],
        };
        let m = model(&ExtractedModel {
            entries: vec![("x".into(), value)],
        });
        assert!(matches!(m.entries.get("r"),Some(ModelEntry::Ref(name,_)) if name == "r"));
        assert_eq!(m.entries.get("x"), m.entries.get("r"));
        let f = ExtractedFunction {
            name: "f".into(),
            argument_types: vec![Sort::Int],
            return_type: Sort::Int,
            options: vec![
                (
                    vec![ExtractedModelEntry::Int(1.into())],
                    ExtractedModelEntry::Int(2.into()),
                ),
                (
                    vec![ExtractedModelEntry::Int(3.into())],
                    ExtractedModelEntry::Int(4.into()),
                ),
            ],
            default: ExtractedModelEntry::Int(0.into()),
        };
        let mapped = functions(&[f], None).unwrap();
        assert_eq!(
            mapped.entries["f"].options[0].1,
            Some(ModelEntry::LitInt("4".into()))
        );
    }
    #[test]
    fn function_variable_result_uses_source_value_resolution() {
        let source_model = ferrum_silver::verifier::verification_error::Model {
            entries: Default::default(),
        };
        let native = NativeCounterexample {
            model: source_model.clone(),
            native_model: source_model,
            store: Arc::new(ferrum_silicon::state::store::MapBackedStore::new()),
            heap: Arc::new(ferrum_silicon::state::heap::ListBackedHeap::new()),
            old_heaps: vec![],
            program: ferrum_silver::ast::program::Program::new(
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                vec![],
                Default::default(),
            ),
            view: NativeCounterexampleView::Native,
        };
        let f = ExtractedFunction {
            name: "f".into(),
            argument_types: vec![],
            return_type: Sort::Ref,
            options: vec![(
                vec![],
                ExtractedModelEntry::Var {
                    name: "r".into(),
                    sort: Sort::Ref,
                },
            )],
            default: ExtractedModelEntry::Int(0.into()),
        };
        let mapped = functions(&[f], Some(&native)).unwrap();
        assert!(
            matches!(&mapped.entries["f"].options[0].1,Some(ModelEntry::Ref(name,fields)) if name=="r" && fields.is_empty())
        );
    }
}
