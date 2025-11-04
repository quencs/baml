function Add() -> int {
    let x = 1;
    let y = {
        let x = 2;
        x + 1;
    };

    x
}

//> Add()
//
// 2
