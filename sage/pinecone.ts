async function main() {
  // await populatePinecone();

  const results = await searchPinecone('what is @alias?');
  console.log(results);

  // await copyPineconeIndex();
}

main();
