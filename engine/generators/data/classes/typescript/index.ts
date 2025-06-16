import { b } from './baml_client'

async function main() {
    const res = await b.MakeSimpleClass()
    console.log(res)
}

await main()