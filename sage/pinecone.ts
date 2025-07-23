import { populatePinecone, searchPinecone } from './app/actions/rag';

async function main() {
  await populatePinecone();
  // await testPopulatePinecone();

  const results = await searchPinecone('is there a baml zed extension?');
  console.log(results);

  // await copyPineconeIndex();
}

main();
