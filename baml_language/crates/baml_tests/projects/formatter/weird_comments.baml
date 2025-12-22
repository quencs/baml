class /*comment*/ Foo{ // comment
field /*comment*/ string@skip // comment
field2 string@skip
field3 /*comment*/string@skip
field4 /*comment*/string@skip
field5/*comment*/ string/*comment*/@skip
field6 string@skip
field7 string@skip
field8 string@skip
field9 /*comment*/string@skip // comment

//comment
field10 string@skip
}

enum /*comment*/ Bar{ // comment
Value1
Value2
}  // comment

client<llm> GPT4
 // comment
{
provider openai
model gpt-4
options{
temperature  /*comment*/ 0.7
}
}

template_string /*comment*/MyPrompt()#"
You are helpful
"# // comment

test  /*comment*/ TestUser /*comment*/ {
functions /*comment*/ [ /*comment*/ GetUser /*comment*/ ]
args/*comment*/{
user_id/*comment*/"123"
}
}
 // comment
type UserId /*comment*/ = /*comment*/ string
 // comment
retry_policy  /*comment*/ DefaultRetry{
max_retries  /*comment*/  /*comment*/ 3
delay_ms  /*comment*/  /*comment*/ 1000
} // comment
 // comment
function HelloWorld(name:string)->string /*comment*/ { /*comment*/  /*comment*/ 
client GPT4
prompt #"
Say hello to{{name}}
"# /*comment*/ 
} // comment