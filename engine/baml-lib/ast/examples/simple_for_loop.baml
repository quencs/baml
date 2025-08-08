# very important header
fn ForLoopWithHeaders() -> int {
    let items = [1, 2, 3, 4, 5];
    let result = 0;
    
    # loop header
    for (item in items) {
        ### Item Processing
        let processed = item * 2;
        
        ### Accumulation
        result = result + processed;
        
        #### Its processing
        processed
    }
    
    # Final Result
    result
}

fn ForLoopWithHeadersTwo() -> int {
    let items = [1, 2, 3, 4, 5];
    let result = 0;
    
    for (item in items) {
        ### Item Processing
        let processed = item * 2;
        
        ### Accumulation
        result = result + processed;
        
        #### Its processing
        processed
    }
    
    result
}


// function Simple() -> int {5}

// # Function with Loops !!!
// function FunctionWithLoops(messages: MyUserMessage[]) -> string {
//   client GPT4o
//   prompt #"
//     {% for message in messages %}
//       {{ message.user_name }}: {{ message.content }}
//     {% endfor %}
//   "#
// }