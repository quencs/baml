import { populatePinecone, searchPinecone } from './app/actions/rag';

async function main() {
  // await populatePinecone();

  const results = await searchPinecone('what is @alias?');
  console.log(results);
}

main();
