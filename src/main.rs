use std::io;
use std::io::Write;
use std::io::Read;
use std::fs::{File, OpenOptions};
use std::fs;
use std::fmt;
use std::process::exit;

mod tests;

#[derive(Debug, Clone, PartialEq)]
struct Col {
    name: String,
    data_type: DataType,
}

#[derive(Debug, Clone, PartialEq)]
struct TableSchema {
    name: String,
    cols: Vec<Col>,
}

// TODO: think about redisign of token system
type Row = Vec<WordType>;

#[derive(Debug, PartialEq)]
struct Table {
    schema: TableSchema,
    rows: Vec<Row>,
}

impl fmt::Display for Table {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(DataType::Count as u8 == 3, "Exhaustive DataType handling in Table::fmt()");
        for Col {name, data_type} in &self.schema.cols {
            match data_type {
                DataType::Int => print!("{name:>5}"),
                DataType::Str => print!("{name:>20}"),
                DataType::Type => print!("{name:>5}"),
                _ => unreachable!(),
            }
        
         }
        println!();
        for row in &self.rows {
            for word in row {
                match word {
                    WordType::Int(value) => print!("{value:>5}"),
                    WordType::Str(value) => print!("{value:>20}"),
                    _ => unreachable!(),
                }
            }
            println!();
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
struct Database {
    name: String,
    tables: Vec<Table>,
}

#[repr(u8)]
#[derive(Debug, PartialEq, Clone)]
enum Op {
    PushWord {
        data_type: DataType, 
        word_type: WordType
    },
    Select,
    Insert,
    Delete,
    FilterAnd,
    FilterOr,
    Equal,
    NotEqual,
    Less,
    More,
    Create,
    DropOp,
    Count,
}

impl Op {
    fn as_u8(&self) -> u8 {
        unsafe { *<*const _>::from(self).cast::<u8>() }
    }
}

// TODO: Introduce a sized string type
#[derive(Debug, PartialEq, Clone)]
enum WordType {
    Int(i32),
    Str(String),
    Type(DataType),
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum DataType {
    Int,
    Str,
    Type,
    Count,
}

fn try_parse_data_type(col_type: &str) -> Option<DataType> {
    assert_eq!(DataType::Count as u8, 3);
    match col_type {
        "Int"  => Some(DataType::Int),
        "Str"  => Some(DataType::Str),
        "Type" => Some(DataType::Type),
        _      => None,
    }
} 

fn data_type_to_string(data_type: DataType) -> String {
    match data_type {
        DataType::Int   => "Int".to_string(),
        DataType::Str   => "Str".to_string(),
        DataType::Type  => "Type".to_string(), 
        DataType::Count => unreachable!(),
    }
}

fn parse_table_schema(file_path: &str) -> Result<TableSchema, String> {
    let file = File::open(file_path);
    if let Err(err) = file {
        return Err(format!("ERROR: unable to open the file {file_path}: {err}"));
    } 
    let mut file = file.unwrap();

    let mut content = String::new();
    if let Err(err) = file.read_to_string(&mut content) {
        return Err(format!("ERROR: unable to read from the file {file_path}: {err}"));
    }

    let mut cols = vec![];
    let mut lines = content.lines();
    let name = match lines.next() {
        Some(value) => value.trim(),
        None => return Err(format!("ERROR: table name not provided in a file {file_path}")),
    };

    if name.len() == 0 {
        return Err(format!("ERROR: table name can't be empty: {file_path}"));
    }

    for (i, line) in lines.enumerate() {
        let (name, type_name) = match line.split_once(':') {
            Some((name, type_name)) => (name.trim(), type_name.trim()),
            None => return Err(format!("ERROR: invalid format for column at line {} in a file: {}", i + 1, file_path)),
        };

        if name.len() == 0 {
            return Err(format!("ERROR: empty column name at line {} in a file {}", i + 1, file_path));
        }

        for Col {name: col_name, ..} in &cols {
            if col_name == name {
                return Err(format!("ERROR: column with name '{}' already exists in table scheme: {}", col_name, file_path));
            } 
        }

        if let Some(value) = try_parse_data_type(type_name) {
            cols.push(Col {
                name: String::from(name), 
                data_type: value
            });
        } else {
            return Err(format!("ERROR: unknown column type at line {} in a file {}", i + 1, file_path));
        } 
    }

    Ok(TableSchema { name: name.to_string(), cols })
}

fn try_parse_op(op: &str) -> Option<Op> {
    assert!(Op::Count.as_u8() == 12, "Exhaustive Op handling in try_parse_op()");
    match op {
        "select"     => Some(Op::Select),
        "insert"     => Some(Op::Insert),
        "delete"     => Some(Op::Delete),
        "filter-and" => Some(Op::FilterAnd),
        "filter-or"  => Some(Op::FilterOr),
        "create"     => Some(Op::Create),
        "drop"       => Some(Op::DropOp),
        "=="         => Some(Op::Equal),
        "!="         => Some(Op::NotEqual),
        ">"          => Some(Op::More),
        "<"          => Some(Op::Less),
        _            => None,  
    }
}

fn parse_query(query: &str) -> Result<Vec<Op>, String> {
    let mut ops: Vec<Op> = vec![];
    let mut query = query.clone();
    loop {
        query = query.trim_start();
        if query.len() == 0 { break; }
        let end = match query.find(char::is_whitespace) {
            Some(end) => end,
            None => query.len(),
        };
        let word = &query[0..end];
        if let Some(op) = try_parse_op(word) {
            ops.push(op); 
            query = &query[end..];
            continue;
        }

        query = query.trim_start_matches(&['(', ')']);
        if query.bytes().next().unwrap() == b'"' {
            query = &query[1..]; 
            if let Some(end) = query.find('"') {
                ops.push(Op::PushWord {
                    data_type: DataType::Str, 
                    word_type: WordType::Str(String::from(&query[0..end]))
                });
                query = &query[end+1..];
            } else {
                return Err(String::from("ERROR: unclosed string literal in a query"));
            }
        } else {
            let end = match query.find(char::is_whitespace) {
                Some(end) => end,
                None => query.len(),
            };
            let mut word = (&query[0..end]).to_string();
            word = word.replace("(", "");
            word = word.replace(")", "");
            query = &query[end..];
            if let Some(op) = try_parse_op(&word) {
                ops.push(op);
            } else if let Some(data_type) = try_parse_data_type(&word) {
                ops.push(Op::PushWord {
                    data_type: DataType::Type, 
                    word_type: WordType::Type(data_type) 
                });
            } else if let Ok(value) = word.parse::<i32>() {
                ops.push(Op::PushWord {
                    data_type: DataType::Int, 
                    word_type: WordType::Int(value) 
                });
            } else {
                ops.push(Op::PushWord {
                    data_type: DataType::Str, 
                    word_type: WordType::Str(String::from(word))
                });
            }
        }
    }
    Ok(ops)
}

#[derive(Debug, PartialEq)]
struct Condition {
    idx: usize,
    value: WordType,
    op: Op,
}

// TODO: Maybe change table with schema
fn logical_op_check(op: Op, words: &[(DataType, WordType)], temp_table: &Table) -> Result<Condition, String> {
    assert!(Op::Count.as_u8()  == 12, "Exhaustive Op handling in logical_op_check()");
    let op_sym = match op {
        Op::Equal => "==",
        Op::NotEqual => "!=",
        Op::Less => "<",
        Op::More => ">",
        _ => unreachable!(),
    };

    if words.len() < 2 {
        return Err(format!("ERROR: not enough arguments for `{op_sym}` operation, provided {0} but needed 2", words.len()));
    }
    
    let col = match &words[words.len() - 2].1 {
        WordType::Str(value) => value.clone(),
        other => return Err(format!("ERROR: invalid argument for `{}` operation, expected string but found {:?}", op_sym, other)),
    };

    let mut idx = -1;
    for (i, Col {name, ..}) in temp_table.schema.cols.iter().enumerate() {
        if *name == col {
            idx = i as i32;
            break;
        }
    }

    if idx < 0 {
        return Err(format!("ERROR: no such column `{0}` in table `{1}`", col, temp_table.schema.name));
    }

    let value = &words[words.len() - 1];
    let col_data_type = temp_table.schema.cols[idx as usize].data_type;
    if value.0 != col_data_type {
        return Err(format!("ERROR: invalid argument for `{}` operation expected type {:?} but found type {:?}", op_sym, col_data_type, value.1));
    }
    
    Ok(Condition {
        idx: idx as usize,
        value: value.1.clone(),
        op: op,
    })
}

fn filter_condition<T: PartialOrd>(a: &T, b: &T, condition: Op) -> bool {
    match condition {
        Op::Equal    => *a != *b,
        Op::NotEqual => *a == *b,
        Op::Less     => *a >= *b,
        Op::More     => *a <= *b,
        _            => unreachable!(),
    }
}

fn execute_query(query: &Vec<Op>, database: &mut Database) -> Option<Table> {
    let mut words: Vec<(DataType, WordType)> = vec![];
    let mut conditions: Vec<Condition> = vec![];
    let mut temp_table = None;    
    for (op_id, op) in query.iter().enumerate() {
        match op {
                Op::Select => {
                    if words.len() == 0 {
                        eprintln!("ERROR: no arguments provided for `select` operation");
                        return None;
                    }
                    let mut row_idxs = vec![];
                    let mut words_iter = words.iter();
                    let table_name = match words_iter.next() {
                        Some((_, name)) => match name {
                            WordType::Str(name) => name.clone(),
                            other => {
                                eprintln!("ERROR: table name expected to be string but found '{:?}'", other);
                                return None;
                            },
                        },
                        None => {
                            eprintln!("ERROR: table name not provided for `select` operation");
                            return None;
                        }
                    };

                    let mut table_idx = -1;
                    for (i, Table {schema, ..}) in database.tables.iter().enumerate() {
                        if table_name == schema.name {
                            table_idx = i as i32;
                            break;
                        }
                    }

                    if table_idx < 0 {
                        eprintln!("ERROR: not such table '{}' in '{}' database", table_name, database.name);
                        return None;
                    }
                    
                    'outer: for (_, word) in words_iter {
                        match word {
                            WordType::Str(value) => {
                                if value == "*" {
                                    row_idxs.append(&mut (0..database.tables[table_idx as usize].schema.cols.len()).collect::<Vec<usize>>());
                                    continue;
                                }

                                for (i, Col {name, ..}) in database.tables[table_idx as usize].schema.cols.iter().enumerate() {
                                    if name == value {
                                        row_idxs.push(i);
                                        continue 'outer;
                                    }
                                }
                                eprintln!("ERROR: non existing column `{0}` in table `{1}`", value, database.tables[table_idx as usize].schema.name);
                                return None;
                            },
                            _ => {
                                eprintln!("ERROR: `select` operation can operate only strings");
                                return None;
                            }
                        }
                    }

                    let mut schema = TableSchema {
                        name: String::from("temp"),
                        cols: vec![],
                    };
                    
                    for idx in &row_idxs {
                        schema.cols.push(database.tables[table_idx as usize].schema.cols[*idx].clone());
                    }

                    temp_table = Some(Table {
                        schema,
                        rows: vec![],
                    });

                    let temp_table = match temp_table {
                        Some(ref mut table) => table,
                        None => unreachable!(),
                    };
                    for row in &database.tables[table_idx as usize].rows {
                        let mut temp_row = vec![];
                        for idx in &row_idxs {
                            temp_row.push(row[*idx as usize].clone());
                        }
                        temp_table.rows.push(temp_row);
                    }
                    words.clear();
                },
                Op::Insert => {
                    if words.len() < 1 {
                        eprintln!("ERROR: table name not provided for `insert` operation");
                        return None;
                    }
                    
                    let table_name = match &words[0].1 {
                        WordType::Str(name) => name.clone(),
                        other => {
                            eprintln!("ERROR: table name expected to be string but found '{:?}'", other);
                            return None;
                        }
                    };
                    
                    let mut table_idx = -1;
                    for (i, Table {schema, ..}) in database.tables.iter().enumerate() {
                        if table_name == schema.name {
                            table_idx = i as i32;
                            break;
                        }
                    }

                    if table_idx < 0 {
                        eprintln!("ERROR: not such table '{}' in '{}' database", table_name, database.name);
                        return None;
                    }
                    
                    let words_slice = &words[1..];
                    let col_count = database.tables[table_idx as usize].schema.cols.len(); 
                    if words_slice.len() < col_count {
                        eprintln!("ERROR: not enaugh arguments for `insert` operation, provided {0} but needed {1}", words_slice.len(), col_count);
                        return None;
                    }

                    for (i, word) in words_slice[words_slice.len() - col_count..words_slice.len()].iter().enumerate() {
                        let col_data_type = database.tables[table_idx as usize].schema.cols[i].data_type;
                        if word.0 != col_data_type {
                            eprintln!("ERROR: argument type don't match the column type, argumnet {0:?}, column {1:?}", word, col_data_type);
                            return None;
                        }
                    }

                    database.tables[table_idx as usize].rows.push(
                        words_slice[words_slice.len() - col_count..words_slice.len()]
                        .iter()
                        .map(|x| x.1.clone())
                        .collect::<Vec<WordType>>());
                    // TODO: delete only used words
                    words.clear();
                },
                Op::Delete => {
                    let mut rows_to_delete = vec![];
                    for (i, row) in database.tables[0].rows.iter().enumerate() {
                        let mut filtered_cols = 0;
                        for condition in &conditions {
                            assert!(Op::Count.as_u8() == 9, "Exhaustive logic Ops handling");
                            match &row[condition.idx] {
                                WordType::Int(value) => {
                                    if let WordType::Int(cond_value) = &condition.value {
                                        if !filter_condition(value, cond_value, condition.op.clone()) {
                                            filtered_cols += 1;
                                        }
                                    } else {
                                        unreachable!();
                                    }
                                },
                                WordType::Str(value) => {
                                    if let WordType::Str(cond_value) = &condition.value {
                                        if !filter_condition(value, cond_value, condition.op.clone()) {
                                            filtered_cols += 1;
                                        }
                                    } else {
                                        unreachable!();
                                    }
                                },
                                _ => unreachable!(),
                            }
                        }
                        if filtered_cols == conditions.len() {
                            rows_to_delete.push(i);
                        }
                    }

                    let mut deleted = 0;
                    for row in rows_to_delete {
                        database.tables[0].rows.remove(row - deleted);
                        deleted += 1;
                    }       
                    conditions.clear();
                },
                Op::FilterAnd => {
                    let mut temp_table = match temp_table {
                        Some(ref mut  table) => table,
                        None => todo!(),
                    };
                    let mut filtered_rows = vec![];
                    for row in &temp_table.rows {
                        let mut filtered = false;
                        for condition in &conditions {
                            assert!(Op::Count.as_u8() == 9, "Exhaustive logic Ops handling");
                            match &row[condition.idx] {
                                WordType::Int(value) => {
                                    if let WordType::Int(cond_value) = &condition.value {
                                        filtered = filter_condition(value, cond_value, condition.op.clone());
                                    } else {
                                        unreachable!();
                                    }
                                },
                                WordType::Str(value) => {
                                    if let WordType::Str(cond_value) = &condition.value {
                                        filtered = filter_condition(value, cond_value, condition.op.clone());
                                    } else {
                                        unreachable!();
                                    }
                                },
                                _ => unreachable!(),
                            }
                            if filtered { break; }
                        }

                        if !filtered {
                            filtered_rows.push(row.clone());
                        }
                    }

                    temp_table.rows = filtered_rows; 
                    conditions.clear();
                },
                Op::FilterOr => {
                    let mut temp_table = match temp_table {
                        Some(ref mut table) => table,
                        None => todo!(),
                    };
                    let mut filtered_rows = vec![];
                    for row in &temp_table.rows {
                        let mut filtered_count = 0;
                        for condition in &conditions {
                            assert!(Op::Count.as_u8() == 9, "Exhaustive logic Ops handling");
                            match &row[condition.idx] {
                                WordType::Int(value) => {
                                    if let WordType::Int(cond_value) = &condition.value {
                                        if filter_condition(value, cond_value, condition.op.clone()) { filtered_count += 1; }
                                    } else {
                                        unreachable!();
                                    }
                                },
                                WordType::Str(value) => {
                                    if let WordType::Str(cond_value) = &condition.value {
                                        if filter_condition(value, cond_value, condition.op.clone()) { filtered_count += 1; }
                                    } else {
                                        unreachable!();
                                    }
                                },
                                _ => unreachable!(),
                            }
                        }

                        if filtered_count < conditions.len() {
                            filtered_rows.push(row.clone());
                        }
                    }

                    temp_table.rows = filtered_rows; 
                    conditions.clear();
                },
                op @ Op::Equal | op @ Op::NotEqual | op @ Op::Less | op @ Op::More => {
                    let mut curr_table = match temp_table {
                        Some(ref table) => table,
                        None => todo!(),
                    };
                    let mut query = query[op_id + 1..].iter();
                    while let Some(op) = query.next() {
                        match op {
                            Op::Delete => {
                                curr_table = &database.tables[0];
                                break;
                            }
                            _ => (),
                        }      
                    } 
                    
                    match logical_op_check(op.clone(), &words, &curr_table) {
                        Ok(condition) => {
                            conditions.push(condition);
                            words.pop();
                            words.pop();
                        },
                        Err(err) => {
                            eprintln!("{}", err);
                            return None;
                        }
                    } 
                },
                Op::Create => {
                    if words.len() == 0 {
                        eprintln!("ERROR: no arguments provided for `create` operation");
                        return None;
                    }

                    let table_name = match &words[0].1 {
                        WordType::Str(name) => name.clone(),
                        other => {
                            eprintln!("ERROR: name of the table expected to be a string but found `{:?}`", other);
                            return None;
                        },
                    };

                    let mut words_iter = words.iter();
                    words_iter.next();
                    let mut cols = vec![];
                    while let Some(word) = words_iter.next() {
                        let col_name = match &word.1 {
                            WordType::Str(name) => name.clone(),
                            other => {
                                eprintln!("ERROR: name of the column expected to be a string but found `{:?}`", other);
                                return None;
                            },
                        };

                        let col_type = match words_iter.next() {
                            Some(data_type) => data_type,
                            None => {
                                eprintln!("ERROR: column type is not provided");
                                return None;
                            },
                        };

                        let col_type = match &col_type.1 {
                            WordType::Type(data_type) => data_type,
                            other => {
                                eprintln!("ERROR: unknown column type '{:?}' in `create` operation", other);
                                return None;
                            },
                        };

                        cols.push(Col {name: col_name, data_type: col_type.clone()});
                    }

                    database.tables.push(Table {
                        schema: TableSchema {
                            name: table_name,
                            cols: cols,
                        },
                        rows: vec![],
                    });
                    words.clear();
                },
                Op::DropOp => {
                    if words.len() < 1 {
                        eprintln!("ERROR: no arguments provided for `drop` operation");
                        return None;
                    }

                    let table_name = match &words[words.len() - 1].1 {
                        WordType::Str(name) => name.clone(),
                        other => {
                            eprintln!("ERROR: name of the table expected to be a string but found `{:?}`", other);
                            return None;
                        },
                    };

                    let mut table_idx = -1; 
                    for (i, table) in database.tables.iter().enumerate() {
                        if table.schema.name == table_name {
                            table_idx = i as i32;
                            break;
                        }
                    }

                    if table_idx < 0 {
                        eprintln!("ERROR: no such table '{}' in database '{}'", table_name, database.name);
                        return None;
                    }

                    database.tables.remove(table_idx as usize);
                },
                Op::PushWord{data_type, word_type} => {
                    words.push((data_type.clone(), word_type.clone())); 
                },
                Op::Count => unreachable!(),
        }
    }

    if words.len() > 0 {
        eprintln!("WARNING: {0} unused words in the stack", words.len());
    }
    if conditions.len() > 0 {
        eprintln!("WARNING: {0} unused conditions in the stack", conditions.len());
    }
    
    temp_table
}

fn read_from_file(dir: &str, schema: TableSchema) -> Table {
    let mut table = Table {
        schema: schema,
        rows: vec![],
    };
    
    let file_path = format!("{}/{}.tbl", dir, table.schema.name);
    let mut file = File::open(&file_path).unwrap_or_else(|_| {
        File::create(&file_path).unwrap_or_else(|err| {
            eprintln!("ERROR: unable to create file {}: {}", file_path, err);
            exit(1);
        })
    });

    let mut row_len = 0;
    for Col {data_type, ..} in &table.schema.cols {
        match data_type {
            DataType::Int => row_len += 4,
            DataType::Str => row_len += 50,
            _ => unreachable!(),
        }
    }
    
    let file_len = fs::metadata(&file_path).unwrap_or_else(|err| {
        eprintln!("ERROR: can't get size of the file {file_path}: {err}");
        exit(1); 
    }).len();
    
    if file_len % row_len != 0 {
        eprintln!("ERROR: incorrect file format in {file_path}");
        exit(1);
    }

    let mut i32_buf: [u8; 4] = [0; 4];
    let mut str_buf: [u8; 50] = [0; 50];
    for _ in 0..file_len / row_len {
        let mut row: Row = vec![];
        for Col {data_type, ..} in &table.schema.cols {
            match data_type {
                DataType::Int => {
                    file.read(&mut i32_buf).unwrap_or_else(|err| {
                        eprintln!("ERROR: unable to read from file {file_path}: {err}");
                        exit(1);
                    });
                    
                    row.push(WordType::Int(i32::from_ne_bytes(i32_buf)));
                },
                DataType::Str => {
                    file.read(&mut str_buf).unwrap_or_else(|err| {
                        eprintln!("ERROR: unable to read from file {file_path}: {err}");
                        exit(1);
                    });
                    
                    let str_len = str_buf.iter().position(|&x| x == 0).unwrap_or(50);
                    row.push(WordType::Str(String::from_utf8_lossy(&str_buf[0..str_len]).to_string()));
                },
                _ => unreachable!(),
            }
        }
        table.rows.push(row);
    }  

    table
}

fn save_to_file(dir: &str, table: &Table) -> Result<(), String> {
    let file_path = format!("{}/{}.tbl", dir, table.schema.name);
    let mut file = match File::create(&file_path) {
        Ok(file) => file,
        Err(err) => return Err(format!("ERROR: unable to create a file for table: {err}")),
    };
    
    for row in &table.rows {
        for word in row {
            match word {
                WordType::Int(value) => {
                    match file.write_all(&value.to_ne_bytes()) {
                        Err(err) => return Err(format!("ERROR: unable to write to the file {file_path}: {err}")),
                        Ok(_) => (),
                    };     
                },
                WordType::Str(value) => {
                    let mut value = &value[0..];
                    if value.len() > 50 {
                        eprintln!("WARNING: string length must be less or equal to 50, only first 50 characters will be saved");
                        value = &value[0..50];
                    }
                    let mut str_buf: [u8; 50] = [0; 50];
                    str_buf[0..value.len()].clone_from_slice(&value.as_bytes());
                    match file.write_all(&str_buf) {
                        Err(err) => return Err(format!("ERROR: unable to write to the file {file_path}: {err}")),
                        Ok(_) => (),
                    };     
                },
                _ => unreachable!(),
            }
        } 
    }

    Ok(())
}

fn load_database_from(path: &str) -> Result<Database, String> {
    let paths = match fs::read_dir(path) {
        Ok(paths) => paths,
        Err(err) => return Err(format!("ERROR: unable to open database directory {}: {}", path, err)),
    };
    
    let mut database = Database {
        name: "database".to_string(),
        tables: vec![],
    };
    
    for file_path in paths {
        let file = format!("{}", match file_path {
            Ok(path) => path,
            Err(err) => return Err(format!("ERROR: something went wrong: {}", err)),
        }.path().display());

        if !file.ends_with(".tbls") {
            continue; 
        }

        let schema = match parse_table_schema(&file) {
            Ok(schema) => schema,
            Err(err) => return Err(err),
        };

        database.tables.push(read_from_file(path, schema)); 
    }

    Ok(database)
}

fn save_schema_to(path: &str, schema: &TableSchema) -> Result<(), String> {
    let path = format!("{}/{}.tbls", path, schema.name);
    let mut file = match OpenOptions::new()
        .write(true)
        .create(true)
        .open(path.clone()) {
            Ok(file) => file,
            Err(err) => return Err(format!("ERROR: couldn't create a file {}: {}", path.clone(), err)),
        };
    
    if let Err(err) = writeln!(file, "{}", schema.name) {
        return Err(format!("ERROR: couldn't write to file {}: {}", path, err));
    } 
   
    for col in &schema.cols {
        if let Err(err) = writeln!(file, "{}:{}", col.name, data_type_to_string(col.data_type)) {
            return Err(format!("ERROR: couldn't write to file {}: {}", path, err));
        } 
    }

    Ok(())
}

fn save_database_to(path: &str, database: &Database) -> Result<(), String> {
    for table in &database.tables {
        if let Err(err) = save_schema_to(path, &table.schema) {
            return Err(err);
        }
        if let Err(err) = save_to_file(path, &table) {
            return Err(err);
        }
    } 

    Ok(())
}

#[derive(PartialEq)]
enum Mode {
    Cmd,
    Query,
    MlQuery,
}

fn main() {
    let mut database = load_database_from("./database").unwrap();

    let mut quit = false;
    let mut mode = Mode::Cmd;
    let mut query = String::new();
    while !quit {
        match mode {
            Mode::Cmd => print!("> "),
            Mode::Query => print!("query > "),
            Mode::MlQuery => print!("query : "),
        } 
        
        io::stdout().flush().unwrap_or_else(|err| {
            eprintln!("ERROR: unable to flush stdout: {err}");
            exit(1);
        });
        
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap_or_else(|err| {
            eprintln!("ERROR: unable to read the line: {err}");
            exit(1);
        });

        // TODO: Add the command history
        match mode {
            Mode::Cmd => {
                let command = buffer.as_str().split_ascii_whitespace().next();
                match command {
                    Some("exit") => quit = true,
                    Some("query") => mode = Mode::Query,
                    None => (),
                    Some(value) => println!("Unknown command: {value}"),
                }
            },
            Mode::Query | Mode::MlQuery => {
                for c in buffer.bytes() {
                    if c == b'(' {
                        mode = Mode::MlQuery;
                    } else if c == b')' {
                        mode = Mode::Query;
                    }
                }
                query.push_str(&buffer);
                if mode == Mode::MlQuery {
                    continue;
                }

                match query.as_str().trim() {
                    "exit" => mode = Mode::Cmd,
                    _ => {
                        match parse_query(query.as_str()) {
                            Ok(tokens) => {
                                if let Some(result_table) = execute_query(&tokens, &mut database) {
                                    print!("{result_table}");
                                }
                            },
                            Err(err) => eprintln!("{}", err),
                        }
                    },
                }
                query.clear();
            },
        }
    }

    if let Err(err) = save_database_to("./database", &database) {
        eprintln!("{}", err);
        exit(1);
    }
}
