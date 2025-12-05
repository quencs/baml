function LlmEcho(input: string) -> string {
  input
}


function MyFunction5(flag: bool) -> string {
  //# Check
  let result = if (flag) {
    "then-branch"
  } else {
    "else-branch"
  };

  //# Extract1
  watch let item = "before";
  LlmEcho(item);

  //# Extract2
  item = "extract2";
  LlmEcho(item);

  //# Extract3
  item = "extract3";
  LlmEcho(item);

  //# Extract4
  item = "extract4";
  LlmEcho(item);

  item
}

function Main() -> string {
  MyFunction5(true)
}

test default {
  functions [Main]
  args { }
}

// Function: LlmEcho
//     0   VIZ_ENTER 0   (node_id=0, log_filter_key=LlmEcho|root:0, type=FunctionRoot, label="LlmEcho")
//
// 1   1   LOAD_VAR 1    (input)
//     2   VIZ_EXIT 0    (node_id=0, log_filter_key=LlmEcho|root:0, type=FunctionRoot, label="LlmEcho")
//     3   RETURN        
//
// String: "then-branch"
// String: "else-branch"
// String: "before"
// String: "item"
// String: "extract2"
// String: "extract3"
// String: "extract4"
// Function: MyFunction5
//      0    VIZ_ENTER 0        (node_id=0, log_filter_key=MyFunction5|root:0, type=FunctionRoot, label="MyFunction5")
//      1    VIZ_ENTER 1        (node_id=1, log_filter_key=MyFunction5|root:0|hdr:check:0, type=HeaderContextEnter, parent_log_filter_key=MyFunction5|root:0, label="Check", level=1)
//
// 7    2    VIZ_ENTER 2        (node_id=2, log_filter_key=MyFunction5|root:0|hdr:check:0|bg:if-flag:0, type=BranchGroup, parent_log_filter_key=MyFunction5|root:0|hdr:check:0, label="if (flag)")
//      3    LOAD_VAR 1         (flag)
//      4    JUMP_IF_FALSE +8   (to 12)
//      5    POP 1              
//      6    VIZ_ENTER 3        (node_id=3, log_filter_key=MyFunction5|root:0|hdr:check:0|bg:if-flag:0|arm:if-flag:0, type=BranchArm, parent_log_filter_key=MyFunction5|root:0|hdr:check:0|bg:if-flag:0, label="if (flag)")
//      7    VIZ_ENTER 4        (node_id=4, log_filter_key=MyFunction5|root:0|hdr:check:0|bg:if-flag:0|arm:else:1, type=BranchArm, parent_log_filter_key=MyFunction5|root:0|hdr:check:0|bg:if-flag:0, label="else")
//
// 8    8    LOAD_CONST 0       (then-branch)
//      9    VIZ_EXIT 4         (node_id=4, log_filter_key=MyFunction5|root:0|hdr:check:0|bg:if-flag:0|arm:else:1, type=BranchArm, parent_log_filter_key=MyFunction5|root:0|hdr:check:0|bg:if-flag:0, label="else")
//      10   VIZ_EXIT 3         (node_id=3, log_filter_key=MyFunction5|root:0|hdr:check:0|bg:if-flag:0|arm:if-flag:0, type=BranchArm, parent_log_filter_key=MyFunction5|root:0|hdr:check:0|bg:if-flag:0, label="if (flag)")
//      11   JUMP +7            (to 18)
//      12   POP 1              
//      13   VIZ_ENTER 5        (node_id=5, log_filter_key=MyFunction5|root:0|hdr:extract1:1, type=HeaderContextEnter, parent_log_filter_key=MyFunction5|root:0, label="Extract1", level=1)
//
// 9    14   VIZ_ENTER 6        (node_id=6, log_filter_key=MyFunction5|root:0|hdr:extract2:2, type=HeaderContextEnter, parent_log_filter_key=MyFunction5|root:0, label="Extract2", level=1)
//
// 10   15   LOAD_CONST 1       (else-branch)
//      16   VIZ_EXIT 6         (node_id=6, log_filter_key=MyFunction5|root:0|hdr:extract2:2, type=HeaderContextEnter, parent_log_filter_key=MyFunction5|root:0, label="Extract2", level=1)
//      17   VIZ_EXIT 5         (node_id=5, log_filter_key=MyFunction5|root:0|hdr:extract1:1, type=HeaderContextEnter, parent_log_filter_key=MyFunction5|root:0, label="Extract1", level=1)
//      18   VIZ_EXIT 2         (node_id=2, log_filter_key=MyFunction5|root:0|hdr:check:0|bg:if-flag:0, type=BranchGroup, parent_log_filter_key=MyFunction5|root:0|hdr:check:0, label="if (flag)")
//      19   VIZ_ENTER 7        (node_id=7, log_filter_key=MyFunction5|root:0|hdr:extract3:3, type=HeaderContextEnter, parent_log_filter_key=MyFunction5|root:0, label="Extract3", level=1)
//
// 14   20   LOAD_CONST 2       (before)
//      21   LOAD_CONST 3       (item)
//      22   LOAD_CONST 4       (null)
//      23   WATCH 3            (item)
//
// 15   24   LOAD_GLOBAL 0      (<fn LlmEcho>)
//      25   LOAD_VAR 3         (item)
//      26   CALL 1             
//      27   POP 1              
//      28   VIZ_ENTER 8        (node_id=8, log_filter_key=MyFunction5|root:0|hdr:extract4:4, type=HeaderContextEnter, parent_log_filter_key=MyFunction5|root:0, label="Extract4", level=1)
//
// 18   29   LOAD_CONST 5       (extract2)
//      30   STORE_VAR 3        (item)
//
// 19   31   LOAD_GLOBAL 0      (<fn LlmEcho>)
//      32   LOAD_VAR 3         (item)
//      33   CALL 1             
//      34   POP 1              
//
// 22   35   LOAD_CONST 6       (extract3)
//      36   STORE_VAR 3        (item)
//
// 23   37   LOAD_GLOBAL 0      (<fn LlmEcho>)
//      38   LOAD_VAR 3         (item)
//      39   CALL 1             
//      40   POP 1              
//
// 26   41   LOAD_CONST 7       (extract4)
//      42   STORE_VAR 3        (item)
//
// 27   43   LOAD_GLOBAL 0      (<fn LlmEcho>)
//      44   LOAD_VAR 3         (item)
//      45   CALL 1             
//      46   POP 1              
//
// 29   47   LOAD_VAR 3         (item)
//      48   VIZ_EXIT 0         (node_id=0, log_filter_key=MyFunction5|root:0, type=FunctionRoot, label="MyFunction5")
//      49   RETURN             
//
// Function: Main
//      0   VIZ_ENTER 0     (node_id=0, log_filter_key=Main|root:0, type=FunctionRoot, label="Main")
//
// 33   1   LOAD_GLOBAL 1   (<fn MyFunction5>)
//      2   LOAD_CONST 0    (true)
//      3   CALL 1          
//      4   VIZ_EXIT 0      (node_id=0, log_filter_key=Main|root:0, type=FunctionRoot, label="Main")
//      5   RETURN          
//
// Class: baml.HttpRequest with 5 fields
// Class: baml.WatchOptions with 2 fields
// Enum baml.HttpMethod
// Function: baml.String.length
//
// Function: baml.Array.length
//
// Function: baml.Array.push
//
// Function: baml.Map.length
//
// Function: baml.Map.has
//
// Function: baml.String.toLowerCase
//
// Function: baml.String.toUpperCase
//
// Function: baml.String.trim
//
// Function: baml.String.includes
//
// Function: baml.String.startsWith
//
// Function: baml.String.endsWith
//
// Function: baml.String.split
//
// Function: baml.String.substring
//
// Function: baml.String.replace
//
// Function: baml.media.image.from_url
//
// Function: baml.media.audio.from_url
//
// Function: baml.media.video.from_url
//
// Function: baml.media.pdf.from_url
//
// Function: baml.media.image.from_base64
//
// Function: baml.media.audio.from_base64
//
// Function: baml.media.video.from_base64
//
// Function: baml.media.pdf.from_base64
//
// Function: baml.media.image.is_url
//
// Function: baml.media.video.is_url
//
// Function: baml.media.audio.is_url
//
// Function: baml.media.pdf.is_url
//
// Function: baml.media.image.is_base64
//
// Function: baml.media.video.is_base64
//
// Function: baml.media.audio.is_base64
//
// Function: baml.media.pdf.is_base64
//
// Function: baml.media.image.as_url
//
// Function: baml.media.video.as_url
//
// Function: baml.media.audio.as_url
//
// Function: baml.media.pdf.as_url
//
// Function: baml.media.image.as_base64
//
// Function: baml.media.video.as_base64
//
// Function: baml.media.audio.as_base64
//
// Function: baml.media.pdf.as_base64
//
// Function: baml.media.image.mime
//
// Function: baml.media.video.mime
//
// Function: baml.media.audio.mime
//
// Function: baml.media.pdf.mime
//
// Function: env.get
//
// Function: baml.deep_copy
//
// Function: baml.deep_equals
//
// Function: baml.unstable.string
//
// Function: baml.fetch_as
