// Pure prop-to-JSON-Schema conversion
// No @/ aliases so builtin-contract.ts can import this directly
import path from 'node:path';
import type {
  ComponentMetaChecker, PropertyMeta, PropertyMetaSchema,
} from 'vue-component-meta';
import {
  makeOptional,
} from '../../shared/types/json-schema';
import type {
  JsonSchema,
} from '../../shared/types/json-schema';

export interface ContractEntry {
  source: string;
  // Each prop value is a JSON Schema object; absent means skip prop validation
  props?: Record<string, JsonSchema>;
}

// Build a component contract from a list of components using a pre-created checker
export function buildContractWithChecker (
  components: Array<{
    name: string;
    absolutePath: string;
  }>,
  projectRoot: string,
  checker: ComponentMetaChecker,
): Record<string, ContractEntry> {
  const contract: Record<string, ContractEntry> = {};

  for (const component of components) {
    const source = `./${path.relative(projectRoot, component.absolutePath)}`;

    try {
      const meta = checker.getComponentMeta(component.absolutePath);
      const props: Record<string, JsonSchema> = {};
      let hasAnyProperty = false;

      for (const property of meta.props) {
        // Skip global HTML attributes injected by vue-component-meta
        if (property.global) continue;

        const schema = toJsonSchema(property.schema);

        if (schema !== undefined) {
          // Wrap in optional (["T","null"]) if the prop is not required and not already nullable
          props[property.name] = property.required ? schema : makeOptional(schema);
          hasAnyProperty = true;
        }
      }

      contract[component.name] = {
        source,
        props: hasAnyProperty ? props : undefined,
      };
    } catch {
      // Component failed to parse: register without props so the name is still known
      contract[component.name] = {
        source,
      };
    }
  }

  return contract;
}

// Homogeneous array -> { type: "array", items: <element> }
function toArray (elements: PropertyMetaSchema[] | undefined): JsonSchema | undefined {
  if (!elements || elements.length !== 1) return undefined;

  const inner = toJsonSchema(elements[0]);

  return inner !== undefined
    ? {
      type: 'array',
      items: inner,
    }
    : undefined;
}

// Strip undefined/null from union, wrap result as optional if they were present
function toEnum (members: PropertyMetaSchema[] | undefined): JsonSchema | undefined {
  const all = members ?? [];
  const isOptional = all.some((member) => member === 'undefined' || member === 'null');
  const concrete = all.filter((member) => member !== 'undefined' && member !== 'null');

  // Only single-member unions after stripping are supported
  if (concrete.length !== 1) return undefined;

  const inner = toJsonSchema(concrete[0]);

  if (inner === undefined) return undefined;

  return isOptional ? makeOptional(inner) : inner;
}

// Dispatch on PropertyMetaSchema kind: string -> primitive, enum/array/object -> recurse
function toJsonSchema (schema: PropertyMetaSchema): JsonSchema | undefined {
  if (typeof schema === 'string') return toPrimitive(schema);

  switch (schema.kind) {
    case 'enum':
      return toEnum(schema.schema);
    case 'array':
      return toArray(schema.schema);
    case 'object':
      return toObject(schema.type, schema.schema);
    default:
      return undefined;
  }
}

// Named fields -> product schema, index signature -> dict schema
function toObject (
  type: string,
  propertySchema: Record<string, PropertyMeta> | undefined,
): JsonSchema | undefined {
  // Has named properties -> product type
  if (propertySchema && 0 < Object.keys(propertySchema).length) {
    return toProductSchema(propertySchema);
  }

  // Regex fallback for index signatures (vue-component-meta lacks structured info)
  const match =
    /^(?:Record<string,\s*(\w+)>|\{\s*\[key:\s*string\]:\s*(\w+);\s*\})$/.exec(type);

  if (!match) return undefined;

  const valueSchema = toPrimitive((match[1] ?? match[2])!);

  return valueSchema !== undefined
    ? {
      type: 'object',
      additionalProperties: valueSchema,
    }
    : undefined;
}

// Map TS primitive name to JSON Schema leaf
function toPrimitive (type: string): JsonSchema | undefined {
  switch (type) {
    case 'string':
      return {
        type: 'string',
      };
    case 'number':
      return {
        type: 'number',
      };
    case 'boolean':
      return {
        type: 'boolean',
      };
    default:
      return undefined;
  }
}

// Recursively map each field, collect required list
function toProductSchema (
  propertySchema: Record<string, PropertyMeta>,
): JsonSchema | undefined {
  const properties: Record<string, JsonSchema> = {};
  const requiredFields: string[] = [];

  for (const [
    key,
    property,
  ] of Object.entries(propertySchema)) {
    const valueSchema = toJsonSchema(property.schema);

    if (valueSchema === undefined) return undefined;

    properties[key] = valueSchema;
    if (property.required) requiredFields.push(key);
  }

  const schema: JsonSchema = {
    type: 'object',
    properties,
    additionalProperties: false,
  };

  if (0 < requiredFields.length) {
    schema.required = requiredFields;
  }

  return schema;
}
