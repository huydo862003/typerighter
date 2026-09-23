---
_label: "The Semantic Web"
_icon: icon.document
---

Link: https://www.w3.org/2000/Talks/0906-xmlweb-tbl/text.htm.

Authors: Tim Berners-Lee, Dan Connolly, Lynn Andrea Stein, Ralph Swick.

Key takeaways:

> 1. Machine-readable web:
>
> - The Semantic Web structures data using frameworks like RDF (Subject-Verb-Object).
> - Machines can understand and process information, acting as a massive, decentralized database rather than just linking documents for humans.
>
> 2. Semantic Web abandons the old Knowledge Representation (KR) goal of building perfect logic engines, prioritizing massive data publication and trusting that powerful navigation tools will evolve to process it.
> 3. Semantic preservation: It breaks down data silos by extracting universal logic from custom systems and using "equivalence statements" to link different terms (e.g, "zipcode" = "postal code") without forcing a single global standard.
> 4. Verifiable trust & proofs: Systems collaborate securely by exchanging mathematical "logical proofs" and digital signatures instead of sharing private internal rules, ensuring every piece of data has a verifiable chain of trust.

## Context

To re-iterate, the **semantic web** is an extension of the **world wide web**. We differentiate the two concepts:

- The world wide web is an interlinked networks of documents, called **hypertexts**.
  - It emphasises on linking related resources together, allowing humans to browse through a network of related resources.
  - Its essence lies in **universality**:
    1. Hypertext link allows anythin to link to anything.
    2. Anything must be able to be put on the web.
  - Implications: The web technology is inclusive, regarding cultures, media, quality.
  - Shortcomings: Machines do not have a well-defined, unambgiuous way to derive meanings from documents in the world wide web.

- The semantic web aims to give data a well-defined meaning so machine can readily process them.

## "Weblike"

- Weblike qualities:
  - Decentralized: No single party control the web content.
  - Universal: Any resources can link to any resources.
  - Compromised "total consistency": Because no single party wholly control the web, no one can ensure assumptions are actually held up.

- Semantic web must be weblike.

- Analogy:
  1. The world wide web is like one big book.
  2. The semantic web is like one big database/mathematical formular.

- The semantic web is not a separate web: It's another dimension of information added to the world wide web.

Before entering the next sections, it's useful to view this tower of semantic web (image taken from `w3.org`):

![Semantic Web Tower](../../../assets/semantic-web-tower.png)

## The First Level of Semantic Web: RDF

- XML (eXtensible Markup Language): Allow creating structured documents that can express various information, but the interpretation of such information is application-specific, which means, there's no universally unambiguous way to describe their meanings.
  - For example, the same `<name>` can mean different things in different contexts.
- RDF (Resource Description Framework): Provide a model to convey meanings of information as sets of **triples** (Subject + Verb + Object). The triples form **directed graphs**, creating a web of concepts.

- RDF treats subjects, verbs and objects as **resources**.
  - Verb is also called an "RDF property".
- In RDF, both subjects and objects are identified by URIs.
  - RDF utilizes **XML namespaces** to assign URIs to every concept. `<name>` in XML now is no longer ambiguous.

- An RDF document asserts that **particular things** have **particular properties** with **particular values**.

## Generalizing About Current Knowledge Representation (KR) Systems

KR is a widely researched topic. The authors identified KR as a field at the time as follows:

> Knowledge Representation as a field is indeed in a state rather similar to the state of Hypertext technology before the web: it is clearly a good idea, and some very nice demonstrations exist, but it has not yet changed the world. In both cases, the idea is good but to be powerful, they must form a single global system. - "The Semantic Web".

### Past Challenges

In the past, KR systems did not scale:

- Some used a centralized data technology, requiring data to be centralized too.
- Some were "conceptually centralized", requiring everyone to maintain the same definition of concepts.
- Some used fuzzy reasoning systems (maybe it means to imply imprecision or best-effort reasoning), but they did not scale.

KR systems that possess weblike qualities should allow concepts to be independently conceived and retrospectively linked.

Another reason worth noting regarding the challenges of **interoperability**:

- Knowledge consists of two parts:
  - Data (facts - easy to share): e.g, "Mary is John's sister".
  - Rules (logic - hard to share): e.g, "A parent's sister is an aunt".
- Systems use a specific program (an inference engine) to apply rules to data and answer questions. Because computers cannot answer infinite or purely general questions, these engines are heavily customized for very specific domains. System creators had to strike a delicate balance when designing their rule languages:
  - Too simple: The system cannot understand or process complex, real-world scenarios.
  - Too powerful: The computer might get stuck in endless loops or fail to guarantee a predictable answer (computational intractability).
- Because every system made different compromises to achieve that balance, their rules became unique to their own engines, making interoperability (sharing rules between different systems) nearly impossible.

### Solution of The Semantic Web

So how does the semantic web resolve this?

- The Semantic Web prioritizes maximum expressive power for now. The Semantic Web completely sidelines the immediate need for computers to process and answer questions about the data.
  - It's making the exact same gamble the early internet did: let people publish a massive, messy ocean of data first, and trust that the tools to navigate it will be invented later.
  - To draw an analogy, detractors of the World Wide Web after its invention pointed out that without a central database and tree structure, it's not possible to find everything. Although it is indeed true, search engines are now remarkably good.
  - So, the creators of the Semantic Web expect the same.

### Challenges of The Semantic Web

The main challenge is building a unified language that can express both the data and the underlying rules of any existing Knowledge Representation (KR) or database system.

- The Semantic Web accepts a hard truth: rules exported from System A probably won't import cleanly into System B.
- Instead, the Semantic Web cares more about **semantic preservation**: It doesn't care about perfect, symmetrical cross-system compatibility. Its core philosophy is simply to get the information out into the open while losing as little meaning as possible.

How does the Semantic Web allow for **semantic preservation**?

- Traditional rules contain both a **universal truth** and a **system-specific instruction manual**.

  As an example: "If you find a daughter, check whether she has a daughter, and if so you have a granddaughter". This has two parts:
  - The logical assertion: The daughter of a daughter is a grandaughter.
  - The hint: The inference engine should use this fact whenever it finds a fact about a daughter.

- The Semantic Web extracts and saves the universal truth (the logic), while completely throwing away the instruction manual (the operational hints).

### A Web of Logic

The current primary task: Giguring out how to successfully inject logic into the web. The logic language must hit a very specific sweet spot:

- _Powerful enough_ to handle complex abstractions (like defining the properties of other properties).
- _Restrictive enough_ to prevent the system from getting trapped in logical paradoxes.

However, the problem sounds harder than it actually is in practice. The vast majority of existing information is basic classification (e.g., "a hex-head-bolt is a type of machine-bolt"). This simple data doesn't stress the system's logic and can be easily handled by existing foundational tools like RDF with just a little extra vocabulary.

Equivalence statements link different names for the same concept across separate databases, breaking down data silos.

- The problem is that unlinked databases treat identical concepts (e.g, "zipcode" vs "postal code") as completely unrelated.
- The solution the semantic web offers is that "equivalence statements" explicitly link these terms, enabling seamless cross-database queries.
- This allows for:
  - Smooth evolution of knowledge based: They can act as built-in translators between old and new versions of standards or vocabularies.
  - Fast, custom local terms to easily map to slower, broader global standards.

### Navigation The Logic Web

Image taken from `w3.org`:

![Semantic Web concepts](../../../assets/semantic-web-concepts.png)

Information isn't found by querying a central database. Systems discover new data by actively following links (URIs) embedded within documents.

When a computer reads a document, it parses the content into a mathematical, logical formula it can process. These logical formulas point to other documents. A system will evaluate if the current, trusted document confers "trust" to the external link before deciding to actually fetch and read it. Because every piece of information is linked back to its origin and the chain of documents that led to it, data always has a **verifiable context**. A system never has to take a random fact at face value without checking its history.

## The Eating

About this, check [the Verification section in the entry note](../../web-3.md#verification).

The Semantic Web doesn't need perfect, universal inference engines to be useful today. Instead of sharing complex internal rules, systems can securely interoperate right now by sharing **logical proofs (not the inference engines)** backed by **digital signatures**:

- Systems with completely different logic engines can still collaborate.
- Scenario:
  1. When System A solves a query, it exports its step-by-step reasoning as a universal mathematical "proof".
  2. System B doesn't need to know how to solve the problem. If it receives a conclusion, it simply asks for the proof, verifies the logic, and decides if it trusts the initial assumptions.

Real-world example: A company (like IBM) doesn't have to expose its private, complex HR rules to an external site (like W3C) to authorize an employee. It simply issues the employee a mathematically verified, digitally signed proof of employment.

Interesting insight: Guaranteeing a computer can solve any complex real-world problem is mathematically intractable. The Semantic Web sidesteps this by not requiring universal problem-solving. Instead, it lets the data flow and relies on a mix of simple, predictable agents and clever heuristic tools (like search engines) to extract practical value.
