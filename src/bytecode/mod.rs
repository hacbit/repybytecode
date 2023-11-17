/*
bytecode用来放python的bytecode指令
一般不会修改这个文件，除非需要增加或者删除部分指令
*/
pub mod bytecode_type;
/*
valuetype用来放python的常见类型
定位和bytecode类似，一般也不会修改这个文件
 */
pub mod valuetype;

/*
用来解释op操作
*/
pub mod op;

pub mod utils;
