# repybytecode

Reverse tools for bytecode of python



## Build

```shell
/$path_to/repybytecode> cargo build --release [--target your_target_os]
```



## Usage

```shell
/$path_to/repybytecode> cargo run help
    Finished dev [unoptimized + debuginfo] target(s) in 0.07s
     Running `target\debug\repybytecode.exe help`
Usage: repybytecode.exe [OPTIONS] --file <FILE> [name] [COMMAND]

Commands:
  test  run the example
  help  Print this message or the help of the given subcommand(s)

Arguments:
  [name]  Optional name

Options:
  -f, --file <FILE>    specify a bytecode file
  -o, --output <FILE>  specify an output file
  -h, --help           Print help
  -V, --version        Print version
```



## Test

```shell
/$path_to/repybytecode> ls test/
Mode                 LastWriteTime         Length Name
----                 -------------         ------ ----
-a---          2023/12/22     0:59             88 for.py
-a---          2023/12/22     0:59           1219 for.txt
-a---          2023/12/21    20:34            112 import.py
-a---          2023/12/21    20:35           1437 import.txt
-a---          2023/12/20    10:58            355 op.py
-a---          2023/12/20    10:58           7057 op.txt
```

**You can run like `repybytecode --file/-f test/import.txt` and compare the result with `import.py` whom generate `import.txt`**

```shell
/$path_to/repybytecode> repybytecode --file ./test/for.txt
 1| arr = [1, 3, 5, 7, 9]
 2| for i, v in enumerate(arr):
 3|     line = i + 1
 4|     print(line, v)
```

**Or run `cargo test` to compare all result generated from *.txt with *.py **

```shell
/$path_to/repybytecode> cargo test

running 3 tests
test bytecode::utils::test::test_import ... ok
test bytecode::utils::test::test_for ... ok
test bytecode::utils::test::test_op ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

     Running unittests src\main.rs (target\debug\deps\repybytecode-996dfc06f4b9f586.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests repybytecode

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

