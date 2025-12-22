class Foo{
field string@skip
field2 string@skip
field3 string@skip
field4 string@skip
field5 string@skip
field6 string@skip
field7 string@skip
field8 string@skip
field9 string@skip
field10 string@skip
}

enum Bar{
Value1
Value2
}

client<llm> GPT4{
provider openai
model gpt-4
options{
temperature 0.7
}
}

template_string MyPrompt()#"
You are helpful
"#

test TestUser{
functions[GetUser]
args{
user_id"123"
}
}

type UserId=string

retry_policy DefaultRetry{
max_retries 3
delay_ms 1000
}

function HelloWorld(name:string)->string{
client GPT4
prompt #"
Say hello to{{name}}
"#
}