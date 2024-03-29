use super::common::*;
use super::opcode::{Opcode, OpcodeInstruction};
use pyrev_ast::*;

pub trait ExprParser {
    fn parse(opcode_instructions: &[OpcodeInstruction]) -> Result<(Box<Self>, TraceBack)>;
}

impl ExprParser for Expr {
    /// 用于解析一段字节码指令为AST
    fn parse(opcode_instructions: &[OpcodeInstruction]) -> Result<(Box<Self>, TraceBack)> {
        let mut exprs_stack = Vec::<ExpressionEnum>::new();
        let mut traceback = TraceBack::new();
        let mut offset = 0;
        loop {
            if offset == opcode_instructions.len() {
                break;
            }
            let instruction = opcode_instructions
                .get(offset)
                .ok_or("[Parse] No instruction")?;
            #[cfg(debug_assertions)]
            {
                //dbg!(&instruction);
            }

            match instruction.opcode {
                Opcode::LoadConst | Opcode::LoadName | Opcode::LoadGlobal => {
                    exprs_stack.push(ExpressionEnum::BaseValue(BaseValue {
                        value: instruction
                            .argval
                            .as_ref()
                            .ok_or(format!(
                                "[Load] No argval, deviation is {}",
                                instruction.offset
                            ))?
                            .clone(),
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                        ..Default::default()
                    }));
                }
                Opcode::LoadFast => {
                    // local variable in function
                    let index = instruction.arg.ok_or(format!(
                        "[LoadFast] No arg, deviation is {}",
                        instruction.offset
                    ))?;
                    let name = instruction
                        .argval
                        .as_ref()
                        .ok_or(format!(
                            "[LoadFast] No argval, deviation is {}",
                            instruction.offset
                        ))?
                        .clone();
                    // 如果先storefast再loadfast, 那么就不认为是函数参数
                    if traceback.locals.contains_key(&index) {
                        traceback.locals.get_mut(&index).unwrap().1 = false;
                    } else {
                        traceback.insert_local(index, name.clone(), false);
                    }

                    exprs_stack.push(ExpressionEnum::BaseValue(BaseValue {
                        value: name,
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                        ..Default::default()
                    }));
                }
                Opcode::LoadAttr => {
                    let parent = exprs_stack.pop().ok_or(format!(
                        "[LoadAttr] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    let attr = instruction.argval.as_ref().ok_or(format!(
                        "[LoadAttr] No argval, deviation is {}",
                        instruction.offset
                    ))?;
                    exprs_stack.push(ExpressionEnum::Attribute(Attribute {
                        parent: Box::new(parent),
                        attr: Box::new(ExpressionEnum::BaseValue(BaseValue {
                            value: attr.clone(),
                            ..Default::default()
                        })),
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                        ..Default::default()
                    }));
                }
                Opcode::LoadMethod => {
                    let parent = exprs_stack.pop().ok_or(format!(
                        "[LoadMethod] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    let method = instruction.argval.as_ref().ok_or(format!(
                        "[LoadMethod] No argval, deviation is {}",
                        instruction.offset
                    ))?;
                    exprs_stack.push(ExpressionEnum::Attribute(Attribute {
                        parent: Box::new(parent),
                        attr: Box::new(ExpressionEnum::BaseValue(BaseValue {
                            value: method.clone(),
                            ..Default::default()
                        })),
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                        ..Default::default()
                    }));
                }
                Opcode::StoreName | Opcode::StoreGlobal => {
                    let name = instruction
                        .argval
                        .as_ref()
                        .ok_or(format!(
                            "[Store] No argval, deviation is {}",
                            instruction.offset
                        ))?
                        .clone();

                    #[cfg(debug_assertions)]
                    {
                        //println!("StoreName argval is {}", name);
                    }

                    let value = exprs_stack.pop().ok_or(format!(
                        "[Store] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;

                    match value {
                        ExpressionEnum::Function(function) => {
                            if function.name.as_str() == "<lambda>"
                                || function.name.as_str() == "<genexpr>"
                                || function.name.as_str() == "<listcomp>"
                            {
                                exprs_stack.push(ExpressionEnum::Assign(Assign {
                                    target: Box::new(ExpressionEnum::BaseValue(BaseValue {
                                        value: name,
                                        ..Default::default()
                                    })),
                                    values: Box::new(ExpressionEnum::Function(function)),
                                    operator: "=".to_string(),
                                    ..Default::default()
                                }));
                            } else {
                                exprs_stack.push(ExpressionEnum::Function(function));
                            }
                        }
                        ExpressionEnum::Import(import) => {
                            if import.bk_module.is_none() {
                                //没from
                                if import.module == name {
                                    //没from，没as
                                    exprs_stack.push(ExpressionEnum::Import(Import {
                                        module: import.module,
                                        bk_module: None,
                                        alias: None,
                                        fragment: None,
                                        start_line: import.start_line,
                                        start_offset: import.start_offset,
                                        end_offset: import.end_offset,
                                    }))
                                } else {
                                    //没from，有as
                                    exprs_stack.push(ExpressionEnum::Import(Import {
                                        module: import.module,
                                        bk_module: None,
                                        alias: Some(name),
                                        fragment: None,
                                        start_line: import.start_line,
                                        start_offset: import.start_offset,
                                        end_offset: import.end_offset,
                                    }))
                                }
                            } else {
                                //有from

                                if import.fragment.as_ref().ok_or(format!(
                                    "[StoreName-import-HaveFrom], deviation is {}",
                                    instruction.offset
                                ))? == &name
                                {
                                    //有from，无as
                                    exprs_stack.push(ExpressionEnum::Import(Import {
                                        module: import.module,
                                        bk_module: Some(
                                            import.bk_module.expect("") + &name.clone() + ", ",
                                        ),
                                        alias: None,
                                        fragment: None,
                                        start_line: import.start_line,
                                        start_offset: import.start_offset,
                                        end_offset: import.end_offset,
                                    }))
                                } else {
                                    //有from，有as
                                    exprs_stack.push(ExpressionEnum::Import(Import {
                                        module: import.module,
                                        bk_module: Some(
                                            import.bk_module.expect("")
                                                + import.fragment.as_ref().ok_or(format!(
                                                    "[StoreName-import-HaveFromAs], deviation is {}",
                                                    instruction.offset
                                                ))?
                                                + " as "
                                                + &name.clone()
                                                + ", ",
                                        ),
                                        alias: None,
                                        fragment: None,
                                        start_line: import.start_line,
                                        start_offset: import.start_offset,
                                        end_offset: import.end_offset,
                                    }))
                                }
                            }
                        }
                        _ => {
                            exprs_stack.push(ExpressionEnum::Assign(Assign {
                                target: Box::new(ExpressionEnum::BaseValue(BaseValue {
                                    value: name,
                                    ..Default::default()
                                })),
                                values: Box::new(value),
                                operator: "=".to_string(),
                                start_offset: instruction.offset,
                                end_offset: instruction.offset,
                                ..Default::default()
                            }));
                        }
                    }
                }
                Opcode::StoreFast => {
                    let name = instruction
                        .argval
                        .as_ref()
                        .ok_or("[StoreFast] No argval")?
                        .clone();
                    let value = exprs_stack.pop().ok_or(format!(
                        "[StoreFast] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    let index = instruction.arg.ok_or(format!(
                        "[StoreFast] No arg, deviation is {}",
                        instruction.offset
                    ))?;
                    traceback.insert_local(index, name.clone(), true);
                    match value {
                        ExpressionEnum::Function(_) => {
                            exprs_stack.push(value);
                        }
                        _ => exprs_stack.push(ExpressionEnum::Assign(Assign {
                            target: Box::new(ExpressionEnum::BaseValue(BaseValue {
                                value: name,
                                ..Default::default()
                            })),
                            values: Box::new(value),
                            operator: "=".to_string(),
                            start_line: instruction.starts_line.unwrap_or_default(),
                            start_offset: instruction.offset,
                            end_offset: instruction.offset,
                        })),
                    }
                }
                Opcode::StoreAttr => {
                    let parent = exprs_stack.pop().ok_or(format!(
                        "[StoreAttr] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    let attr = instruction.argval.as_ref().ok_or(format!(
                        "[StoreAttr] No argval, deviation is {}",
                        instruction.offset
                    ))?;
                    let value = exprs_stack.pop().ok_or(format!(
                        "[StoreAttr] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    exprs_stack.push(ExpressionEnum::Assign(Assign {
                        target: Box::new(ExpressionEnum::Attribute(Attribute {
                            parent: Box::new(parent),
                            attr: Box::new(ExpressionEnum::BaseValue(BaseValue {
                                value: attr.clone(),
                                ..Default::default()
                            })),
                            start_offset: instruction.offset,
                            end_offset: instruction.offset,
                            ..Default::default()
                        })),
                        values: Box::new(value),
                        operator: "=".to_string(),
                        start_line: instruction.starts_line.unwrap_or_default(),
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                    }));
                }
                Opcode::LoadBuildClass => {
                    let mark = opcode_instructions
                        .get(offset + 1)
                        .cloned()
                        .ok_or(format!(
                            "[LoadBuildClass] No mark, deviation is {}",
                            instruction.offset
                        ))?
                        .argval
                        .ok_or(format!(
                            "[LoadBuildClass] No argval, deviation is {}",
                            instruction.offset
                        ))?;
                    exprs_stack.push(ExpressionEnum::Class(Class::new(mark)?));

                    break;
                }
                Opcode::FormatValue => {
                    let format_value = exprs_stack.pop().ok_or(format!(
                        "[FormatValue] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    exprs_stack.push(ExpressionEnum::FormatValue(FormatValue {
                        value: Box::new(format_value),
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                        ..Default::default()
                    }));
                }
                Opcode::BuildString => {
                    let size = instruction.arg.ok_or(format!(
                        "[BuildString] No arg, deviation is {}",
                        instruction.offset
                    ))?;
                    let mut format_string = Vec::with_capacity(size);
                    for _ in 0..size {
                        format_string.push(exprs_stack.pop().ok_or(format!(
                            "[BuildString] Stack is empty, deviation is {}",
                            instruction.offset
                        ))?);
                    }
                    format_string.reverse();
                    exprs_stack.push(ExpressionEnum::Format(Format {
                        format_values: format_string,
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                        ..Default::default()
                    }));
                }
                Opcode::BuildTuple => {
                    let size = instruction.arg.ok_or(format!(
                        "[BuildTuple] No arg, deviation is {}",
                        instruction.offset
                    ))?;
                    let mut tuple = Vec::with_capacity(size);
                    for _ in 0..size {
                        tuple.push(exprs_stack.pop().ok_or(format!(
                            "[BuildTuple] Stack is empty, deviation is {}",
                            instruction.offset
                        ))?);
                    }
                    tuple.reverse();
                    exprs_stack.push(ExpressionEnum::Container(Container {
                        values: tuple,
                        container_type: ContainerType::Tuple,
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                        ..Default::default()
                    }));
                }
                Opcode::BuildList => {
                    let size = instruction.arg.ok_or(format!(
                        "[BuildList] No arg, deviation is {}",
                        instruction.offset
                    ))?;
                    if size == 0 {
                        exprs_stack.push(ExpressionEnum::Container(Container {
                            values: vec![],
                            container_type: ContainerType::List,
                            start_offset: instruction.offset,
                            end_offset: instruction.offset,
                            ..Default::default()
                        }));
                    } else {
                        let mut list = Vec::with_capacity(size);
                        for _ in 0..size {
                            list.push(exprs_stack.pop().ok_or(format!(
                                "[BuildList] Stack is empty, deviation is {}",
                                instruction.offset
                            ))?);
                        }
                        list.reverse();
                        exprs_stack.push(ExpressionEnum::Container(Container {
                            values: list,
                            container_type: ContainerType::List,
                            start_offset: instruction.offset,
                            end_offset: instruction.offset,
                            ..Default::default()
                        }));
                    }
                }
                Opcode::ListExtend => {
                    let size = instruction.arg.ok_or(format!(
                        "[ListExtend] No arg, deviation is {}",
                        instruction.offset
                    ))?;
                    let mut extend = Vec::with_capacity(size);
                    for _ in 0..size {
                        extend.push(exprs_stack.pop().ok_or(format!(
                            "[ListExtend] Stack is empty, deviation is {}",
                            instruction.offset
                        ))?);
                    }
                    extend.reverse();
                    let list = exprs_stack.pop().ok_or(format!(
                        "[ListExtend] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    if let ExpressionEnum::Container(Container {
                        values: mut list,
                        container_type: ContainerType::List,
                        ..
                    }) = list
                    {
                        extend.iter_mut().for_each(|x| {
                            if let ExpressionEnum::BaseValue(value) = x {
                                value.value = value
                                    .value
                                    .trim_start_matches('(')
                                    .trim_end_matches(')')
                                    .to_string();
                            }
                        });
                        list.append(&mut extend);
                        exprs_stack.push(ExpressionEnum::Container(Container {
                            values: list,
                            container_type: ContainerType::List,
                            start_offset: instruction.offset,
                            end_offset: instruction.offset,
                            ..Default::default()
                        }));
                    } else {
                        return Err("[ListExtend] Invalid list".into());
                    }
                }
                Opcode::BuildSet => {
                    let size = instruction.arg.ok_or(format!(
                        "[BuildSet] No arg, deviation is {}",
                        instruction.offset
                    ))?;
                    let mut set = Vec::with_capacity(size);
                    for _ in 0..size {
                        set.push(exprs_stack.pop().ok_or(format!(
                            "[BuildSet] Stack is empty, deviation is {}",
                            instruction.offset
                        ))?);
                    }
                    set.reverse();
                    exprs_stack.push(ExpressionEnum::Container(Container {
                        values: set,
                        container_type: ContainerType::Set,
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                        ..Default::default()
                    }));
                }
                Opcode::BuildMap => {
                    let size = instruction.arg.ok_or(format!(
                        "[BuildMap] No arg, deviation is {}",
                        instruction.offset
                    ))?;
                    if size == 0 {
                        exprs_stack.push(ExpressionEnum::Container(Container {
                            values: vec![],
                            container_type: ContainerType::Dict,
                            start_offset: instruction.offset,
                            end_offset: instruction.offset,
                            ..Default::default()
                        }));
                    } else {
                        let mut map = Vec::with_capacity(size * 2);
                        for _ in 0..size {
                            let value = exprs_stack.pop().ok_or(format!(
                                "[BuildMap] Stack is empty, deviation is {}",
                                instruction.offset
                            ))?;
                            let key = exprs_stack.pop().ok_or(format!(
                                "[BuildMap] Stack is empty, deviation is {}",
                                instruction.offset
                            ))?;
                            map.push(value);
                            map.push(key);
                        }
                        map.reverse();
                        exprs_stack.push(ExpressionEnum::Container(Container {
                            values: map,
                            container_type: ContainerType::Dict,
                            start_offset: instruction.offset,
                            end_offset: instruction.offset,
                            ..Default::default()
                        }));
                    }
                }
                Opcode::BuildConstKeyMap => {
                    let size = instruction.arg.ok_or(format!(
                        "[BuildConstKeyMap] No arg, deviation is {}",
                        instruction.offset
                    ))?;
                    let keys = exprs_stack.pop().ok_or(format!(
                        "[BuildConstKeyMap] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    let mut values = Vec::with_capacity(size);
                    for _ in 0..size {
                        values.push(exprs_stack.pop().ok_or(format!(
                            "[BuildConstKeyMap] Stack is empty, deviation is {}",
                            instruction.offset
                        ))?);
                    }
                    values.reverse();
                    let mut map = Vec::with_capacity(size * 2);
                    if let ExpressionEnum::BaseValue(BaseValue { value: key, .. }) = keys {
                        for (k, v) in key
                            .trim_start_matches('(')
                            .trim_end_matches(')')
                            .split(", ")
                            .zip(values)
                        {
                            map.push(ExpressionEnum::BaseValue(BaseValue {
                                value: k.to_string(),
                                ..Default::default()
                            }));
                            map.push(v);
                        }
                    }
                    exprs_stack.push(ExpressionEnum::Container(Container {
                        values: map,
                        container_type: ContainerType::Dict,
                        ..Default::default()
                    }))
                }
                Opcode::BuildSlice => {
                    let size = instruction.arg.ok_or(format!(
                        "[BuildSlice] No arg, deviation is {}",
                        instruction.offset
                    ))?;
                    let mut slice = Vec::with_capacity(size);
                    for _ in 0..size {
                        slice.push(exprs_stack.pop().ok_or(format!(
                            "[BuildSlice] Stack is empty, deviation is {}",
                            instruction.offset
                        ))?);
                    }
                    slice.reverse();
                    let origin = exprs_stack.pop().ok_or(format!(
                        "[BuildSlice] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    exprs_stack.push(ExpressionEnum::Slice(Slice {
                        origin: Box::new(origin),
                        slice,
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                        ..Default::default()
                    }));
                }
                Opcode::MakeFunction => {
                    let mark = exprs_stack.pop().ok_or(format!(
                        "[MakeFunction] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    let mut function = Function::from(mark)?;
                    if let Some(argval) = instruction.argval.as_ref() {
                        if argval.contains("annotations") {
                            let values = exprs_stack.pop().ok_or(format!(
                                "[MakeFunction] Stack is empty, deviation is {}",
                                instruction.offset
                            ))?;
                            if let ExpressionEnum::Container(container) = values {
                                #[cfg(debug_assertions)]
                                {
                                    assert_eq!(container.container_type, ContainerType::Tuple);
                                }

                                for (idx, exprs) in container.values.chunks(2).enumerate() {
                                    if exprs.iter().all(|e| e.is_base_value()) {
                                        let arg = FastVariable {
                                            index: idx,
                                            name: exprs
                                                .first()
                                                .unwrap()
                                                .unwrap_base_value()
                                                .value
                                                .trim_start_matches('\'')
                                                .trim_end_matches('\'')
                                                .to_string(),
                                            annotation: Some(
                                                exprs
                                                    .get(1)
                                                    .as_ref()
                                                    .unwrap()
                                                    .unwrap_base_value()
                                                    .value,
                                            ),
                                            ..Default::default()
                                        };
                                        function.args.push(arg);
                                    }
                                }
                            }
                        }
                        if argval.contains("defaults") {
                            let defaults = exprs_stack.pop().ok_or(format!(
                                "[MakeFunction] Stack is empty, deviation is {}",
                                instruction.offset
                            ))?;
                            if let ExpressionEnum::BaseValue(BaseValue { value, .. }) = defaults {
                                let defaults = value
                                    .trim_start_matches('(')
                                    .trim_end_matches(')')
                                    .trim_end_matches(',')
                                    .split(", ")
                                    .map(|x| x.to_string())
                                    .collect::<Vec<_>>();
                                function.defaults = defaults;
                            }
                        }
                    }
                    function.start_offset = instruction.offset;
                    function.end_offset = instruction.offset;
                    exprs_stack.push(ExpressionEnum::Function(function));
                }
                // BinaryOperation
                Opcode::BinaryOp | Opcode::CompareOp => {
                    let right = exprs_stack.pop().ok_or(format!(
                        "[BinaryOp] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    let left = exprs_stack.pop().ok_or(format!(
                        "[BinaryOp] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    exprs_stack.push(ExpressionEnum::BinaryOperation(BinaryOperation {
                        left: Box::new(left),
                        right: Box::new(right),
                        operator: instruction
                            .argval
                            .as_ref()
                            .ok_or("[BinaryOp] No argval")?
                            .clone(),
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                        ..Default::default()
                    }))
                }
                // BinaryOperation
                Opcode::IsOp => {
                    let right = exprs_stack.pop().ok_or(format!(
                        "[IsOp] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    let left = exprs_stack.pop().ok_or(format!(
                        "[IsOp] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    let operator = match instruction.arg.as_ref() {
                        Some(0) => "is",
                        Some(1) => "is not",
                        _ => return Err("[IsOp] No arg or Invalid arg".into()),
                    };
                    exprs_stack.push(ExpressionEnum::BinaryOperation(BinaryOperation {
                        left: Box::new(left),
                        right: Box::new(right),
                        operator: operator.to_string(),
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                        ..Default::default()
                    }))
                }
                // BinaryOperation
                Opcode::ContainsOp => {
                    let right = exprs_stack.pop().ok_or(format!(
                        "[ContainsOp] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    let left = exprs_stack.pop().ok_or(format!(
                        "[ContainsOp] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    let operator = match instruction.arg.as_ref() {
                        Some(0) => "in",
                        Some(1) => "not in",
                        _ => return Err("[ContainsOp] No arg or Invalid arg".into()),
                    };
                    exprs_stack.push(ExpressionEnum::BinaryOperation(BinaryOperation {
                        left: Box::new(left),
                        right: Box::new(right),
                        operator: operator.to_string(),
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                        ..Default::default()
                    }))
                }
                Opcode::UnaryInvert => {
                    let target = exprs_stack.pop().ok_or(format!(
                        "[UnaryInvert] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    exprs_stack.push(ExpressionEnum::UnaryOperation(UnaryOperation {
                        target: Box::new(target),
                        unary_type: UnaryType::Invert,
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                        ..Default::default()
                    }))
                }
                Opcode::UnaryNegative => {
                    let target = exprs_stack.pop().ok_or(format!(
                        "[UnaryNegative] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    exprs_stack.push(ExpressionEnum::UnaryOperation(UnaryOperation {
                        target: Box::new(target),
                        unary_type: UnaryType::Negative,
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                        ..Default::default()
                    }))
                }
                Opcode::UnaryNot => {
                    let target = exprs_stack.pop().ok_or(format!(
                        "[UnaryNot] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    exprs_stack.push(ExpressionEnum::UnaryOperation(UnaryOperation {
                        target: Box::new(target),
                        unary_type: UnaryType::Not,
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                        ..Default::default()
                    }))
                }
                Opcode::Call => {
                    #[cfg(debug_assertions)]
                    {
                        //dbg!(&exprs_stack);
                    }

                    let count = instruction.arg.ok_or(format!(
                        "[Call] No arg, deviation is {}",
                        instruction.offset
                    ))?;
                    if count == 0 {
                        let last = exprs_stack.pop().ok_or(format!(
                            "[Call] Stack is empty, deviation is {}",
                            instruction.offset
                        ))?;
                        if let ExpressionEnum::BaseValue(base_value) = &last {
                            if base_value.value.contains(' ') {
                                // not a function call
                                exprs_stack.push(last);
                            } else {
                                exprs_stack.push(ExpressionEnum::Call(Call {
                                    func: Box::new(last),
                                    args: vec![],
                                    start_offset: instruction.offset,
                                    end_offset: instruction.offset,
                                    ..Default::default()
                                }))
                            }
                        } else {
                            exprs_stack.push(ExpressionEnum::Call(Call {
                                func: Box::new(last),
                                args: vec![],
                                start_offset: instruction.offset,
                                end_offset: instruction.offset,
                                ..Default::default()
                            }));
                        }
                        offset += 1;
                        continue;
                    }
                    let mut args = Vec::with_capacity(count);
                    for _ in 0..count {
                        args.push(exprs_stack.pop().ok_or(format!(
                            "[Call] Stack is empty, deviation is {}",
                            instruction.offset
                        ))?);
                    }
                    args.reverse();
                    match exprs_stack.pop() {
                        Some(ExpressionEnum::BaseValue(function_name)) => {
                            //dbg!(&function_name);
                            let function_name = function_name.value.trim_start_matches("NULL + ");
                            exprs_stack.push(ExpressionEnum::Call(Call {
                                func: Box::new(ExpressionEnum::BaseValue(BaseValue {
                                    value: function_name.to_string(),
                                    ..Default::default()
                                })),
                                args,
                                start_line: instruction.starts_line.unwrap_or_default(),
                                start_offset: instruction.offset,
                                end_offset: instruction.offset,
                            }))
                        }
                        Some(function) => exprs_stack.push(ExpressionEnum::Call(Call {
                            func: Box::new(function),
                            args,
                            start_line: instruction.starts_line.unwrap_or_default(),
                            start_offset: instruction.offset,
                            end_offset: instruction.offset,
                        })),
                        None => return Err("[Call] Stack is empty".into()),
                    }
                }
                Opcode::ReturnValue => {
                    //dbg!(&exprs_stack);
                    let value = exprs_stack.pop().ok_or(format!(
                        "[ReturnValue] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    exprs_stack.push(ExpressionEnum::Return(Return {
                        value: Box::new(value),
                        start_line: instruction.starts_line.unwrap_or_default(),
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                    }));
                }
                Opcode::YieldValue => {
                    let value = exprs_stack.pop().ok_or(format!(
                        "[YieldValue] Stack is empty, deviation is {}",
                        instruction.offset,
                    ))?;
                    exprs_stack.push(ExpressionEnum::Yield(Yield {
                        value: Box::new(value),
                        start_line: instruction.starts_line.unwrap_or_default(),
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                    }))
                }
                Opcode::ImportFrom => {
                    let value = exprs_stack.pop().ok_or(format!(
                        "[ImportFrom] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    if let ExpressionEnum::Import(import) = value {
                        if import.bk_module.is_none() {
                            //没from
                            import.with_mut().patch_by(|i| i.fragment = None).unwrap();
                            exprs_stack.push(ExpressionEnum::Import(import))
                        } else {
                            //有from
                            let new_fragment = Some(
                                instruction
                                    .argval
                                    .as_ref()
                                    .ok_or(format!(
                                        "[ImportName] No argval, deviation is {}",
                                        instruction.offset
                                    ))?
                                    .clone(),
                            );
                            import
                                .with_mut()
                                .patch_by(|i| i.fragment = new_fragment)
                                .unwrap();
                            exprs_stack.push(ExpressionEnum::Import(import))
                        }
                    }
                }
                Opcode::ImportName => {
                    let module = exprs_stack.pop().ok_or(format!(
                        "[ImportName] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;

                    if !module.build()?.join("").is_empty() {
                        //需要from
                        let origin_len = module.build()?.join("").len();
                        if origin_len                  //有“ * ”
                        - module
                            .build()?
                            .join("")
                            .replace('*', "")
                            .len()
                            == 1
                        {
                            exprs_stack.push(ExpressionEnum::Import(Import {
                                module: instruction
                                    .argval
                                    .as_ref()
                                    .ok_or(format!(
                                        "[ImportName] No argval, deviation is {}",
                                        instruction.offset
                                    ))?
                                    .clone(),
                                bk_module: Some('*'.to_string()),
                                fragment: None,
                                alias: None,
                                start_line: instruction.starts_line.unwrap_or_default(),
                                start_offset: instruction.offset,
                                end_offset: instruction.offset,
                            }))
                        } else {
                            //没 “ * ”
                            exprs_stack.push(ExpressionEnum::Import(Import {
                                module: instruction
                                    .argval
                                    .as_ref()
                                    .ok_or(format!(
                                        "[ImportName] No argval, deviation is {}",
                                        instruction.offset
                                    ))?
                                    .clone(),
                                bk_module: Some("".to_string()),
                                fragment: None,
                                alias: None,
                                start_line: instruction.starts_line.unwrap_or_default(),
                                start_offset: instruction.offset,
                                end_offset: instruction.offset,
                            }))
                        }
                    } else {
                        //不需要from
                        exprs_stack.push(ExpressionEnum::Import(Import {
                            module: instruction
                                .argval
                                .as_ref()
                                .ok_or(format!(
                                    "[ImportName] No argval, deviation is {}",
                                    instruction.offset
                                ))?
                                .clone(),
                            bk_module: None,
                            fragment: None,
                            alias: None,
                            start_line: instruction.starts_line.unwrap_or_default(),
                            start_offset: instruction.offset,
                            end_offset: instruction.offset,
                        }))
                    }
                }
                Opcode::PopJumpIfTrue => {
                    if let Some(next_instruction) = opcode_instructions.get(offset + 1) {
                        // if next instruction is LoadAssertionError, then it's an assert
                        if next_instruction.opcode == Opcode::LoadAssertionError {
                            offset += 1;
                            continue;
                        }
                    }

                    let test = exprs_stack.pop().ok_or(format!(
                        "[PopJumpIfTrue] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    let test = ExpressionEnum::UnaryOperation(UnaryOperation {
                        target: Box::new(test),
                        unary_type: UnaryType::Not,
                        ..Default::default()
                    });
                    let jump_target = instruction
                        .argval
                        .as_ref()
                        .ok_or(format!(
                            "[PopJumpIfTrue] No argval, deviation is {}",
                            instruction.offset
                        ))?
                        .trim_start_matches("to ")
                        .parse::<usize>()?;
                    exprs_stack.push(ExpressionEnum::If(If {
                        test: Box::new(test),
                        body: vec![],
                        or_else: vec![ExpressionEnum::Jump(Jump {
                            target: jump_target,
                            body: vec![],
                            start_line: instruction.starts_line.unwrap_or_default(),
                            start_offset: instruction.offset,
                            end_offset: instruction.offset,
                        })],
                        start_line: instruction.starts_line.unwrap_or_default(),
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                    }));
                }
                Opcode::PopJumpIfFalse => {
                    if let Some(next_instruction) = opcode_instructions.get(offset + 1) {
                        // if next instruction is LoadAssertionError, then it's an assert
                        if next_instruction.opcode == Opcode::LoadAssertionError {
                            offset += 1;
                            continue;
                        }
                    }

                    let test = exprs_stack.pop().ok_or(format!(
                        "[PopJumpIfFalse] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    let jump_target = instruction
                        .argval
                        .as_ref()
                        .ok_or(format!(
                            "[PopJumpIfFalse] No argval, deviation is {}",
                            instruction.offset
                        ))?
                        .trim_start_matches("to ")
                        .parse::<usize>()?;
                    exprs_stack.push(ExpressionEnum::If(If {
                        test: Box::new(test),
                        body: vec![],
                        or_else: vec![ExpressionEnum::Jump(Jump {
                            target: jump_target,
                            body: vec![],
                            start_line: instruction.starts_line.unwrap_or_default(),
                            start_offset: instruction.offset,
                            end_offset: instruction.offset,
                        })],
                        start_line: instruction.starts_line.unwrap_or_default(),
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                    }));
                }
                Opcode::JumpForward => {
                    let jump_target = instruction
                        .argval
                        .as_ref()
                        .ok_or(format!(
                            "[JumpForward] No argval, deviation is {}",
                            instruction.offset
                        ))?
                        .trim_start_matches("to ")
                        .parse::<usize>()?;
                    exprs_stack.push(ExpressionEnum::Jump(Jump {
                        target: jump_target,
                        body: vec![],
                        start_line: instruction.starts_line.unwrap_or_default(),
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                    }));
                }
                Opcode::LoadAssertionError => {
                    let test = exprs_stack.pop().ok_or(format!(
                        "[LoadAssertionError] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;

                    exprs_stack.push(ExpressionEnum::Assert(Assert {
                        test: Box::new(test),
                        msg: None,
                        start_line: instruction.starts_line.unwrap_or_default(),
                        start_offset: instruction.offset,
                        end_offset: instruction.offset,
                    }))
                }
                Opcode::RaiseVarargs => {
                    let expr = exprs_stack.pop().ok_or(format!(
                        "[RaiseVarargs] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    if expr.is_base_value() {
                        let exception = expr;
                        if let Some(expr) = exprs_stack.pop() {
                            if let Ok(assert) = expr.query_singleton::<Assert>() {
                                assert
                                    .with_mut()
                                    .patch_by(|a| a.msg = Some(Box::new(exception)))?;
                            }
                            exprs_stack.push(expr);
                        } else {
                            exprs_stack.push(ExpressionEnum::Raise(Raise {
                                exception: Box::new(exception),
                                start_line: instruction.starts_line.unwrap_or_default(),
                                start_offset: instruction.offset,
                                end_offset: instruction.offset,
                            }))
                        }
                    } else {
                        #[cfg(debug_assertions)]
                        {
                            assert!(expr.is_assert());
                        }
                        //
                    }
                }
                Opcode::CheckExcMatch => {
                    let err = exprs_stack.pop().ok_or(format!(
                        "[CheckExcMatch] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    if let Some(store_name_instruction) = opcode_instructions[offset..]
                        .iter()
                        .find(|x| x.opcode == Opcode::StoreName)
                    {
                        let alias = store_name_instruction.argval.as_ref().ok_or(format!(
                            "[CheckExcMatch] No argval, deviation is {}",
                            instruction.offset
                        ))?;

                        exprs_stack.push(ExpressionEnum::Except(Except {
                            exception: Box::new(ExpressionEnum::Alias(Alias {
                                target: Box::new(err),
                                alias: Box::new(ExpressionEnum::BaseValue(BaseValue {
                                    value: alias.clone(),
                                    ..Default::default()
                                })),
                                ..Default::default()
                            })),
                            body: vec![],
                            start_line: instruction.starts_line.unwrap_or_default(),
                            start_offset: instruction.offset,
                            end_offset: instruction.offset,
                        }));
                    } else {
                        exprs_stack.push(ExpressionEnum::Except(Except {
                            exception: Box::new(err),
                            body: vec![],
                            start_line: instruction.starts_line.unwrap_or_default(),
                            start_offset: instruction.offset,
                            end_offset: instruction.offset,
                        }))
                    }
                }
                Opcode::BeforeWith => {
                    let expr = exprs_stack.pop().ok_or(format!(
                        "[BeforeWith] Stack is empty, deviation is {}",
                        instruction.offset
                    ))?;
                    let mut with = With {
                        start_line: instruction.starts_line.unwrap_or_default(),
                        start_offset: instruction.offset,
                        ..Default::default()
                    };
                    if let Some(next_instruction) = opcode_instructions.get(offset + 1) {
                        if next_instruction.opcode == Opcode::StoreName {
                            let name = next_instruction.argval.as_ref().ok_or(format!(
                                "[BeforeWith] No argval, deviation is {}",
                                instruction.offset
                            ))?;
                            with.item = Box::new(ExpressionEnum::Alias(Alias {
                                target: Box::new(expr),
                                alias: Box::new(ExpressionEnum::BaseValue(BaseValue {
                                    value: name.clone(),
                                    ..Default::default()
                                })),
                                ..Default::default()
                            }));
                        } else {
                            with.item = Box::new(expr);
                        }
                    }
                    if let Some(next_instruction) = opcode_instructions.get(offset + 2) {
                        with.end_offset = next_instruction.offset;
                    }
                    exprs_stack.push(ExpressionEnum::With(with));
                    break;
                }
                Opcode::ForIter => {
                    let iter = exprs_stack.pop().ok_or(format!(
                        "[ForIter] Stack is empty, deviation is {}",
                        instruction.offset,
                    ))?;
                    let jump_target = instruction
                        .argval
                        .as_ref()
                        .ok_or(format!(
                            "[ForIter] No argval, deviation is {}",
                            instruction.offset
                        ))?
                        .split("to ")
                        .last()
                        .ok_or(format!(
                            "[ForIter] Invalid argval, expect \"to /* usize */\", but got {:?}",
                            instruction.argval.as_ref().unwrap()
                        ))?
                        .parse::<usize>()?;
                    if let Some(next_instruction) = opcode_instructions.get(offset + 1) {
                        if next_instruction.opcode == Opcode::StoreName
                            || next_instruction.opcode == Opcode::StoreFast
                        {
                            exprs_stack.push(ExpressionEnum::For(For {
                                iterator: Box::new(iter),
                                items: Box::new(ExpressionEnum::BaseValue(BaseValue {
                                    value: next_instruction
                                        .argval
                                        .as_ref()
                                        .ok_or(format!(
                                            "[ForIter] No argval, deviation is {}",
                                            next_instruction.offset
                                        ))?
                                        .clone(),
                                    ..Default::default()
                                })),
                                from: instruction.offset,
                                to: jump_target,
                                start_offset: instruction.offset,
                                end_offset: instruction.offset,
                                ..Default::default()
                            }));
                            offset += 1;
                        } else {
                            exprs_stack.push(ExpressionEnum::For(For {
                                iterator: Box::new(iter),
                                from: instruction.offset,
                                to: jump_target,
                                start_offset: instruction.offset,
                                end_offset: instruction.offset,
                                ..Default::default()
                            }))
                        }
                    } else {
                        return Err(format!(
                            "[ForIter] No next instruction, deviation is {}",
                            instruction.offset,
                        )
                        .into());
                    }
                }
                Opcode::UnpackSequence => {
                    let mut count = instruction.arg.ok_or(format!(
                        "[UnpackSequence] No arg, deviation is {}",
                        instruction.offset
                    ))?;
                    let mut sequence = Vec::with_capacity(count);
                    loop {
                        if count == 0 {
                            break;
                        }
                        let next_instruction =
                            opcode_instructions.get(offset + 1).ok_or(format!(
                                "[UnpackSequence] No next instruction, diviation is {}",
                                instruction.offset
                            ))?;
                        match next_instruction.opcode {
                            Opcode::StoreName => {
                                let name = next_instruction
                                    .argval
                                    .as_ref()
                                    .ok_or(format!(
                                        "[UnpackSequence] No argval, diviation is {}",
                                        next_instruction.offset
                                    ))?
                                    .clone();
                                if let Some(ExpressionEnum::Container(sub_seq)) = sequence.last() {
                                    if sub_seq.values.len() < sub_seq.values.capacity() {
                                        sub_seq.with_mut().patch_by(|container| {
                                            container.values.push(ExpressionEnum::BaseValue(
                                                BaseValue {
                                                    value: name,
                                                    start_offset: next_instruction.offset,
                                                    end_offset: next_instruction.offset,
                                                    ..Default::default()
                                                },
                                            ))
                                        })?;
                                    }
                                    if sub_seq.values.len() == sub_seq.values.capacity() {
                                        count -= 1;
                                    }
                                } else {
                                    sequence.push(ExpressionEnum::BaseValue(BaseValue {
                                        value: name,
                                        start_offset: next_instruction.offset,
                                        end_offset: next_instruction.offset,
                                        ..Default::default()
                                    }));
                                    count -= 1;
                                }
                            }
                            Opcode::UnpackSequence => {
                                let sub_count = next_instruction.arg.ok_or(format!(
                                    "[UnpackSequence] No arg, diviation is {}",
                                    next_instruction.offset
                                ))?;
                                let sub_sequence = Vec::with_capacity(sub_count);
                                sequence.push(ExpressionEnum::Container(Container {
                                    values: sub_sequence,
                                    start_offset: next_instruction.offset,
                                    end_offset: next_instruction.offset,
                                    ..Default::default()
                                }))
                            }
                            _ => {
                                return Err(format!(
                                "[UnpackSequence] Expect StoreName or UnpackSequence, but got {:?}",
                                next_instruction.opcode
                            )
                                .into())
                            }
                        }
                        offset += 1;
                    }
                    if let Some(ExpressionEnum::For(for_expr)) = exprs_stack.last() {
                        for_expr.with_mut().patch_by(|f| {
                            f.items = Box::new(ExpressionEnum::Container(Container {
                                values: sequence,
                                start_offset: instruction.offset,
                                end_offset: instruction.offset,
                                ..Default::default()
                            }))
                        })?;
                    }
                }

                _ => {}
            }

            offset += 1;
        }

        Ok((Box::new(Self { bodys: exprs_stack }), traceback))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_expr() {
        let instructions = [
            OpcodeInstruction {
                opcode: Opcode::LoadConst,
                opname: "LOAD_CONST".into(),
                arg: Some(0),
                argval: Some("'a'".into()),
                offset: 2,
                starts_line: Some(1),
                is_jump_target: false,
                positions: vec![],
            },
            OpcodeInstruction {
                opcode: Opcode::LoadName,
                opname: "LOAD_NAME".into(),
                arg: Some(0),
                argval: Some("int".into()),
                offset: 4,
                starts_line: Some(1),
                is_jump_target: false,
                positions: vec![],
            },
            OpcodeInstruction {
                opcode: Opcode::LoadConst,
                opname: "LOAD_CONST".into(),
                arg: Some(1),
                argval: Some("'return'".into()),
                offset: 6,
                starts_line: Some(1),
                is_jump_target: false,
                positions: vec![],
            },
            OpcodeInstruction {
                opcode: Opcode::LoadName,
                opname: "LOAD_NAME".into(),
                arg: Some(0),
                argval: Some("int".into()),
                offset: 8,
                starts_line: Some(1),
                is_jump_target: false,
                positions: vec![],
            },
            OpcodeInstruction {
                opcode: Opcode::BuildTuple,
                opname: "BUILD_TUPLE".into(),
                arg: Some(4),
                argval: None,
                offset: 10,
                starts_line: Some(1),
                is_jump_target: false,
                positions: vec![],
            },
            OpcodeInstruction {
                opcode: Opcode::LoadConst,
                opname: "LOAD_CONST".into(),
                arg: Some(2),
                argval: Some(
                    "<code object test at 0x00000279922BDB80, file \"test/def.py\", line 1>".into(),
                ),
                offset: 12,
                starts_line: Some(1),
                is_jump_target: false,
                positions: vec![],
            },
            OpcodeInstruction {
                opcode: Opcode::MakeFunction,
                opname: "MAKE_FUNCTION".into(),
                arg: Some(4),
                argval: Some("annotations".into()),
                offset: 14,
                starts_line: Some(1),
                is_jump_target: false,
                positions: vec![],
            },
            OpcodeInstruction {
                opcode: Opcode::StoreName,
                opname: "STORE_NAME".into(),
                arg: Some(1),
                argval: Some("test".into()),
                offset: 16,
                starts_line: Some(1),
                is_jump_target: false,
                positions: vec![],
            },
        ];

        assert_eq!(
            Expr::parse(&instructions).unwrap().0,
            Box::new(Expr {
                bodys: [ExpressionEnum::Function(Function {
                    mark: "<code object test at 0x00000279922BDB80, file \"test/def.py\", line 1>"
                        .into(),
                    name: "test".into(),
                    args: [
                        FastVariable {
                            index: 0,
                            name: "a".into(),
                            annotation: Some("int".into()),
                            ..Default::default()
                        },
                        FastVariable {
                            index: 1,
                            name: "return".into(),
                            annotation: Some("int".into()),
                            ..Default::default()
                        },
                    ]
                    .into(),
                    defaults: vec![],
                    start_line: 1,
                    end_line: 1,
                    bodys: vec![],
                    start_offset: 14,
                    end_offset: 14,
                    ..Default::default()
                },),]
                .into()
            })
        )
    }

    #[test]
    fn test_query() {
        let expr = Box::new(Expr {
            bodys: [ExpressionEnum::Assign(Assign {
                target: Box::new(ExpressionEnum::BaseValue(BaseValue {
                    value: "test".into(),
                    ..Default::default()
                })),
                values: Box::new(ExpressionEnum::Function(Function {
                    mark: "<code object test at 0x00000279922BDB80, file \"test/def.py\", line 1>"
                        .into(),
                    name: "test".into(),
                    args: vec![],
                    defaults: vec![],
                    start_line: 1,
                    end_line: 1,
                    bodys: vec![],
                    ..Default::default()
                })),
                operator: "=".into(),
                ..Default::default()
            })]
            .into(),
        });
        let assign_query = expr.query::<Assign>();
        let function_query = expr.query::<Function>();
        //dbg!(query);
        //assert!(false);
        assert_eq!(
            assign_query,
            vec![&Assign {
                target: Box::new(ExpressionEnum::BaseValue(BaseValue {
                    value: "test".into(),
                    ..Default::default()
                })),
                values: Box::new(ExpressionEnum::Function(Function {
                    mark: "<code object test at 0x00000279922BDB80, file \"test/def.py\", line 1>"
                        .into(),
                    name: "test".into(),
                    args: vec![],
                    defaults: vec![],
                    start_line: 1,
                    end_line: 1,
                    bodys: vec![],
                    ..Default::default()
                },)),
                operator: "=".into(),
                ..Default::default()
            }]
        );
        assert_eq!(
            function_query,
            vec![&Function {
                mark: "<code object test at 0x00000279922BDB80, file \"test/def.py\", line 1>"
                    .into(),
                name: "test".into(),
                args: vec![],
                defaults: vec![],
                start_line: 1,
                end_line: 1,
                bodys: vec![],
                ..Default::default()
            }]
        );
    }

    #[test]
    fn test_any() {
        let expr = Assign {
            target: Box::new(ExpressionEnum::BaseValue(BaseValue {
                value: "a".to_string(),
                ..Default::default()
            })),
            values: Box::new(ExpressionEnum::BaseValue(BaseValue {
                value: "1".to_string(),
                ..Default::default()
            })),
            operator: "=".to_string(),
            ..Default::default()
        };
        let any = expr.try_query::<Assign>();

        //dbg!(any);
        //assert!(false);
        assert_eq!(
            any,
            Some(&Assign {
                target: Box::new(ExpressionEnum::BaseValue(BaseValue {
                    value: "a".to_string(),
                    ..Default::default()
                })),
                values: Box::new(ExpressionEnum::BaseValue(BaseValue {
                    value: "1".to_string(),
                    ..Default::default()
                })),
                operator: "=".to_string(),
                ..Default::default()
            })
        )
    }
}
