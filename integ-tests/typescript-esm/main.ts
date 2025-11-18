import { b } from "./baml_client/index.js";
import { Pdf } from "@boundaryml/baml";
import { TypeBuilder } from "./baml_client/type_builder.js";
// import { Image} from "@boundaryml/baml";
// Force another import for the logging path.
import { setLogLevel } from "./baml_client/config.js";


setLogLevel("info");

const pdf = Pdf.fromBase64("JVBERi0K...")
const tb = new TypeBuilder()
// const image = Image.fromBase64("image/png", "iVBORw0K...")

// const result = await b.DescribeImage(Image.fromUrl('https://upload.wikimedia.org/wikipedia/en/4/4d/Shrek_%28character%29.png'));

// console.log(result);
