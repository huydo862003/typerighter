---
_label: "Semantic Web"
_icon: icon.document
---

This is the main page of the Semantic web research, containing sub-links to sub-researches.

As a document itself, this goes on about the overview of semantic webs (web 3): their philosophy, practices & recurring themes/ideas.

Key takeaways:

> - The semantic web is an **extension** of the world wide web, focusing on defining a consistent, unambiguous, machine-comprehensible semantic model of the world wide web components.
> - The semantic web has 4 main components:
>   - XML: To define custom entities. Entities are defined uniquely by URIs.
>   - RDF: Define relationship between entities.
>   - Ontology: A taxonomy for entities and inference rules based on the relationship between entities. An ontology can also define equivalence relations of its concepts to other ontologies' concepts.

Here is the list of resources used this document:

- [The Semantic Web Article (Berners-Lee et al.)](https://www-sop.inria.fr/acacia/cours/essi2006/Scientific%20American_%20Feature%20Article_%20The%20Semantic%20Web_%20May%202001.pdf).

See related resources:

- [The Semantic Web Slides by Berners-Lee et al.](https://www.w3.org/2000/Talks/0906-xmlweb-tbl/text.htm). Check the corresponding notes here: [Link](./web-3/the-semantic-web.md)
- [The Semantic Web Made Easy](https://www.w3.org/RDF/Metalog/docs/sw-easy). Check the corresponding notes here: [Link](./web-3/the-semantic-web-made-easy.md)

Refer to [Research Topics](#research-topics) for the more in-depth details of the semantic web.

## Other Relevant Resources

- [W3C Homepage](https://www.w3.org/standards/)
  - [W3C Semantic Web Overview](https://www.w3.org/standards/semanticweb/)
  - [W3C Semantic Web Wiki](https://www.w3.org/2001/sw/wiki/Main_Page)
  - [W3C RDF Primer](https://www.w3.org/TR/rdf11-primer/)
  - [W3C OWL Overview](https://www.w3.org/TR/owl2-overview/)
  - [W3C SPARQL Spec](https://www.w3.org/TR/sparql11-overview/)
- [Linked Data Principles](https://www.w3.org/DesignIssues/LinkedData.html) - Tim Berners-Lee
- [Schema.org](https://schema.org/)
- _Semantic Web for the Working Ontologist_ - Dean Allemang, James Hendler (Morgan Kaufmann)

## Expected Topics to Research

- The philosophy of Semantic Web.
- The model of Semantic Web.
- RDF.
- Ontology.
- Query and processing Semantic Web.

## Overview

> A new form of Web content that is meaningful to computers will unleash a revolution of new possibilities. - "The Semantic Web" article by Tim Berners Lee.

Semantic web aims to add a machine-readable semantic layer to the largely unstructured traditional web, which was mostly aimed towards human. It incorporates a suite of philosophy and technology to make the traditional web easilly analyzable by tractable algorithms. This, in turn, allows the machine/web crawlers/search engines to construct a graph of information found on the web.

Note that the traditional web is already a graph, however, the nodes in this graph is an opaque web page from which useful knowledge can be somewhat extracted. With the semantic web though, the nodes are more fine-grained: it's no longer working at the grain of web pages anymore - it's now working at the grain of knowledge the web pages contain.

Check out this example on an imagined scenario and possible use case that the semantic web enabled (written by Tim Berners Lee - the web standard creator): [Link (First section)](https://www-sop.inria.fr/acacia/cours/essi2006/Scientific%20American_%20Feature%20Article_%20The%20Semantic%20Web_%20May%202001.pdf).

- Each person has a "hand-held" web-browser and a personal web agent.
- Each website also provides an agent.
- Agents can work collaboratively on the structured information that the semantic web offer to make informed decision.
- The interaction between the human and the agent seems largely explorative (like our modern-day interaction with AI agents).

## Motivation of The Semantic Web

- The Wold Wide Web today:
  1. Reliably parse Web pages.
  2. Reliably detect web page layouts and routine processing.
  3. Cannot process the semantics of the web page.

- The Semantic Web:
  1. Bring structure to the unstructured web content (HTML, etc.).
  2. Enable roaming agents to carry out sophisticated tasks.
  3. Lighter-weight than the current sophisticated AI but still well-informed.

- Tim Berner's Lee vision:
  - Developers can use "off-the-shell" software to write Semantic Web pages.
  - Normal users have agents roaming the pages to make well-informed decision on behalf of the user.

## Definition

> The Semantic Web is not a separate Web but an extension of the current one, in which information is given well-defined meaning, better enabling computers and people to work in cooperation.

## Design Philosophy

> The Semantic Web is not a separate Web but an extension of the current one, in which information is given well-defined meaning, better enabling computers and people to work in cooperation. - "The Semantic Web" article by Tim Berners Lee.

The Semantic Web is an **extension** of the existing Web, not a **replacement**. Essentially, it's a **semantic layer**/enhancement/augmentation over the traditional web.

Infrastructure is already partially in place; the near-term goal is to make machines capable of processing the data they currently only display.

The Web's core property is **universality**: anything can link to anything, without discrimination between document types, cultures, or media. So far, this **universality** has mostly served human-readable documents (pages, articles, media). The Semantic Web extends this to machine-processable data.

Like the Internet, the Semantic Web is designed to be **decentralized**, trading **total consistency** for unchecked growth. The trade-off is familiar: "404 Not Found" is the cost of allowing anyone to publish anything without central coordination.

> **Typedown doesn't really prioritize decentralized (notes are mostly centralized) & favor total consistency more.**

According to [Knowledge Representation](#knowledge-representation), essentially, the semantic web unify the language to specify documents/knowledge and the language to define rues for reasoning about the data in which anyone can publish document.

## Knowledge Representation

Knowledge representation is the act of representing the knowledge base in a machine-readable fashion.

Traditional knowledge-representation systems are centralized and rigid:

- Everyone must share the same definitions.
- Rules are narrow and idiosyncratic.
- Questions are carefully limited to avoid unanswerable queries.

However, similar to Kurt Gödel's theorem ("any system that is complex enough to be useful also encompasses unanswerable questions"), the trandition knowledge-representation systems are highly restricted.

Semantic Web accepts a different trade-off: **paradoxes and unanswerable questions are unavoidable**, but this enables **versatility and openness**. Any system can export its rules onto the Web without central coordination.

> We make the language for the rules as expressive as needed to allow the Web to reason as widely as desired. - "The Semantic Web" article by Tim Berners Lee.

### Challenge

The challenge of semantic web is "to provide a language that expresses both data and rules for reasoning about the data and that allows rules from any existing knowledge-representation system to be exported onto the Web".

- The logic expressible should cover most complex properties of objects.
- The logic must be limited to not express paradox.

> Fortunately, a large majority of the information we want to express is along the lines of "a hex-head bolt is a type of machine bolt," which is readily written in existing languages with a little extra vocabulary. - "The Semantic Web" article by Tim Berners Lee.

### Key Technologies

Related technologies: XML & RDF.

Two key technologies enable this:

- **XML**: adds arbitrary structure to documents via custom tags (e.g., `<zip_code>`). Structure without meaning.
  - Kind of like [YAML](./yaml-1.2.md).
  - The YAML only describe data. Semantics and data types are encoded in both the YAML processor and language runtime.
- **RDF**: encodes meaning as triples (subject, verb, object), like elementary sentences. Each part identified by a URI, so concepts are globally unambiguous.

URIs solve the fundamental problem: human language tolerates ambiguity ("address" can mean mailing address, street address, or speech). Machines do not and URI is used to tie a concept to a unique definition that everyone can find on the web. Using distinct URIs for each concept (`mailing_address` vs `street_address`) prevents confusion when data crosses system boundaries.

In essence:

- RDF triples form webs of information.
- URIs ensure concepts are tied to unique, discoverable definitions on the Web.

## Ontologies (The Third Semantic Web Component)

RDF and URIs are not the end of the story. To motivate the concept of "ontology":

1. RDF and URIs solve local ambiguity, but a broader problem remains: different databases may independently define similar concepts with different identifiers.
2. A program needs to know that `zip_code` (in one system) and `postal_code` (in another) refer to the same thing. Without this knowledge, systems cannot meaningfully combine information across sources.

In philosophy, an ontology defines what types of things exist & ontology as a discipline studies about these.

To AI and Web researchers, "an ontology is a **document** or **file** that formally defines the **relations** among **terms**". Basically, an ontology is a schema, like YAML schema.

The most typical types of web ontologies often involve a **taxonomy** and **a set of inference rules**.

### How Ontologies Solve It

Ontologies are **documents** that formally define **relations** among terms.

They provide:

- **Taxonomies**: Class hierarchies (classes and subclasses) and relations.

  For example:
  - An address is a type of location.
  - City codes apply only to locations.

  > "If city codes must be of type city and cities generally have Web sites, we can discuss the Web site associated with a city code even if no database links a city code directly to a Web site."

  Classes can inherit properties from parent classes, reducing redundancy.

- **Inference rules**: Encode logical deductions.

  Example: "If a city code is associated with a state, and an address uses that city code, then the address is in that state". A computer can then deduce that a Cornell University address in Ithaca must be in New York, which is in the U.S.

- **Equivalence relations**: map concepts across ontologies. If you publish an ontology defining "zip code" and I publish one defining "postal code", either or both ontologies can state: `my:zip_code` is equivalent to `your:postal_code`.

## Agents (The Application of The Samantic Web)

**Related technology**: [DAML](http://www.w3.org/2000/Talks/0815-daml-sweb-tbl) (DARPA Agent Markup Language).

Because the Semantic Web aims at making web pages more machine-readable, **software agents** that harvest data from diverse sources, process it, and exchange results emerge.

Note that the above 3 components of the Semantic Web define the infrastructure. Agents are not really a component of the semantic web structure itself, if I understand correctly.

The power grows exponentially as machine-readable content increases. The **core idea** is that: agents designed independently can collaborate when data carries explicit semantics - structured meaning lets them understand and act on unfamiliar data.

### Verification

The web is decentralized and so are the agents. Therefore, it's very possible that agents can fabricate the data or trying to spoof as some other party.

Regarding this, there are two key aspects of agents: **Proof verification** and **Digital signature**.

- Digitial signature: Used to verify that the sources (knowledges) come from legit parties.
- Proof verification: Used to verify that legit claims are inferred based on the sources.

**Proof verification**:

- Agents build trust by exchanging **proofs** in Semantic Web's unifying language.
- Example:
  1. Your agent finds Ms. Cook in Johannesburg via an online service.
  2. You ask the service for proof.
  3. It translates its internal reasoning to Semantic Web language.
  4. Your inference engine verifies the chain of logic and shows source pages.

This enables verification without human intervention.

**Digital signatures** verify source authenticity: Encrypted data blocks prove information came from trusted source-not forged.

- Example: verify an accounting statement from retailer is genuine, not spoofed by neighbor.
- Agents must verify sources before trusting assertions on Semantic Web.

### Service Discovery

Agents need to find services performing specific functions.

**Service discovery** requires common language describing what service does and how to use it. Services advertise in directories (like Yellow Pages).

Current schemes (UPnP - a schema for NAT traversal, Jini) work at syntax level only, rely on pre-standardized function descriptions. The problem is that standards can't anticipate all future needs (_not sure what is really implied here?_). Semantic Web enables open discovery through meaningful service descriptions any agent can understand.

Semantic Web is more flexible compared to the previous schemes:

- Agents exchange ontologies for shared understanding, allowing a shared vocabulary.
- Agents can bootstrap new reasoning when discovering new ontologies.
- Semantics enables partial matches, exploiting services even if they don't perfectly fit requests.

### Value Chains

Agents form **value chains**: data passes agent-to-agent, each adds value.

Distributed Web data transformed progressively into high-value output for user.

Example: agents find providers, filter by insurance plan, match appointments to user schedule. Raw data (worthless scattered across Web) -> refined result (actionable plan).
