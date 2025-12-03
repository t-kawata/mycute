## About Cognee

Cognee is an open-source tool and platform that transforms your raw data into persistent and dynamic AI memory for Agents. It combines vector search with graph databases to make your documents both searchable by meaning and connected by relationships. 

### How to run the Pipeline

Cognee will take your documents, generate a knowledge graph from them and then query the graph based on combined relationships. 

```python
import cognee
import asyncio


async def main():
    # Add text to cognee
    await cognee.add("Cognee turns documents into AI memory.")

    # Generate the knowledge graph
    await cognee.cognify()

    # Add memory algorithms to the graph
    await cognee.memify()

    # Query the knowledge graph
    results = await cognee.search("What does Cognee do?")

    # Display the results
    for result in results:
        print(result)


if __name__ == '__main__':
    asyncio.run(main())

```

As you can see, the output is generated from the document we previously stored in Cognee:

```bash
  Cognee turns documents into AI memory.
```
