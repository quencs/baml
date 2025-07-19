function ForLoopWithHeaders() -> int {
    # Loop Processing
    let items = [1, 2, 3, 4, 5];
    let result = 0;
    
    ## Main Loop
    for (item in items) {
        ### Item Processing
        let processed = item * 2;
        
        #### Accumulation
        result = result + processed;
    }
    
    ## Final Result
    result
}

// warning: Workflow functions are experimental, and will break in the future.
//   -->  expr/mdx_headers_for_loops.baml:1
//    | 
//    | 
//  1 | function ForLoopWithHeaders() -> int {
//    | 
