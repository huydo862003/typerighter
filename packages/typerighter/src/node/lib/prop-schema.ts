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

export interface ComponentPropertyEntry {
  schema: JsonSchema;
  description?: string;
}

export interface ContractEntry {
  source: string;
  props?: Record<string, ComponentPropertyEntry>;
  slots?: string[];
  deprecated?: string;
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
      const props: Record<string, ComponentPropertyEntry> = {};
      let hasAnyProperty = false;

      for (const property of meta.props) {
        // Skip global HTML attributes injected by vue-component-meta
        if (property.global) continue;

        const schema = convertPropertyMetaToSchema(property.schema);

        if (schema !== undefined) {
          const entry: ComponentPropertyEntry = {
            schema: property.required ? schema : makeOptional(schema),
          };

          if (property.description) {
            entry.description = property.description;
          }
          props[property.name] = entry;
          hasAnyProperty = true;
        }
      }

      const slotNames = meta.slots
        .map((slot) => slot.name)
        .filter((name) => name !== 'default');

      contract[component.name] = {
        source,
        props: hasAnyProperty ? props : undefined,
        slots: 0 < slotNames.length ? slotNames : undefined,
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
function convertArrayElementsToSchema (elements: PropertyMetaSchema[] | undefined): JsonSchema | undefined {
  if (!elements || elements.length !== 1) return undefined;

  const inner = convertPropertyMetaToSchema(elements[0]);

  return inner !== undefined
    ? {
      type: 'array',
      items: inner,
    }
    : undefined;
}

// Named fields -> product schema, index signature -> dict schema
function convertObjectToSchema (
  type: string,
  propertySchema: Record<string, PropertyMeta> | undefined,
): JsonSchema | undefined {
  // Has named properties -> product type
  if (propertySchema && 0 < Object.keys(propertySchema).length) {
    return convertPropertiesToProductSchema(propertySchema);
  }

  // Regex fallback for index signatures (vue-component-meta lacks structured info)
  const match =
    /^(?:Record<string,\s*(\w+)>|\{\s*\[key:\s*string\]:\s*(\w+);\s*\})$/.exec(type);

  if (!match) return undefined;

  const valueSchema = convertPrimitiveTypeToSchema((match[1] ?? match[2])!);

  return valueSchema !== undefined
    ? {
      type: 'object',
      additionalProperties: valueSchema,
    }
    : undefined;
}

// Map TS primitive name to JSON Schema leaf
function convertPrimitiveTypeToSchema (type: string): JsonSchema | undefined {
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
function convertPropertiesToProductSchema (
  propertySchema: Record<string, PropertyMeta>,
): JsonSchema | undefined {
  const properties: Record<string, JsonSchema> = {};
  const requiredFields: string[] = [];

  for (const [
    key,
    property,
  ] of Object.entries(propertySchema)) {
    const valueSchema = convertPropertyMetaToSchema(property.schema);

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

// Dispatch on PropertyMetaSchema kind: string -> primitive, enum/array/object -> recurse
function convertPropertyMetaToSchema (schema: PropertyMetaSchema): JsonSchema | undefined {
  if (typeof schema === 'string') return convertPrimitiveTypeToSchema(schema);

  switch (schema.kind) {
    case 'enum':
      return convertUnionMembersToSchema(schema.schema);
    case 'array':
      return convertArrayElementsToSchema(schema.schema);
    case 'object':
      return convertObjectToSchema(schema.type, schema.schema);
    default:
      return undefined;
  }
}

// Strip undefined/null from union, handle single-type and literal enum unions
function convertUnionMembersToSchema (members: PropertyMetaSchema[] | undefined): JsonSchema | undefined {
  const all = members ?? [];
  const isNullable = all.some((member) => member === 'undefined' || member === 'null');
  const concrete = all.filter((member) => member !== 'undefined' && member !== 'null');

  if (concrete.length === 0) return undefined;

  // Single-member union after stripping nullability
  if (concrete.length === 1) {
    const inner = convertPropertyMetaToSchema(concrete[0]);

    if (inner === undefined) return undefined;

    return isNullable ? makeOptional(inner) : inner;
  }

  // All members are literal values: emit { enum: [...] }
  const literals = concrete.every(
    (member) => typeof member === 'object' && member.kind === 'literal',
  );

  if (literals) {
    const values: (string | number | boolean)[] = [];

    for (const member of concrete) {
      // kind === 'literal' always has value, but guard defensively
      if (typeof member === 'string' || !('value' in member)) return undefined;
      values.push(member.value as string | number | boolean);
    }
    const schema: JsonSchema = {
      enum: values,
    };

    return isNullable ? makeOptional(schema) : schema;
  }

  return undefined;
}
