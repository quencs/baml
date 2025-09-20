import { b } from "./baml_client/index.js";
import { Image } from "@boundaryml/baml";
import { setLogLevel } from "./baml_client/config.js";

setLogLevel("info");

const result = await b.DescribeImage(
  Image.fromUrl(
    "https://upload.wikimedia.org/wikipedia/en/4/4d/Shrek_%28character%29.png",
  ),
);

console.log(result);
