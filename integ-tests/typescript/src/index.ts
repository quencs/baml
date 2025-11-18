import { Pdf } from "@boundaryml/baml";
import { b } from "../baml_client";
import { TypeBuilder } from "../baml_client/type_builder";

const pdf = Pdf.fromBase64("JVBERi0K...")
const tb = new TypeBuilder();
console.log(pdf)

async function main() {
  const request = await b.request.AaaSamOutputFormat("blah");
  console.log(request.body.json())
}

main()

export { };
