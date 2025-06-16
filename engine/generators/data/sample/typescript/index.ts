import { b } from '../baml_client'

//

async function main() {
    const res = await b.TestFnNamedArgsSingleClass({
        key: 'key',
        key_two: true,
        key_three: 52,
    })
    console.log(res)
}

main()