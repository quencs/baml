import type { FieldType } from "../native";

export type Type = FieldType;
export type Typeish = Type;

type IsLiteral<T extends string> = string extends T ? false : true;
type NameOf<
  T extends string,
  Default extends string
> = IsLiteral<T> extends true ? T : Default;

type NameOfClass<
  ClassName extends string,
  Default extends string = "DynamicClass"
> = NameOf<ClassName, Default>;
type NameOfProperty<
  PropertyName extends string,
  Default extends string = "dynamicProperty"
> = NameOf<PropertyName, Default>;
type NameOfClassProperty<
  ClassName extends string,
  PropertyName extends string
> = `${NameOfClass<ClassName>}.${NameOfProperty<PropertyName>}`;

type NameOfEnum<EnumName extends string> = NameOf<EnumName, "DynamicEnum">;
type NameOfEnumValue<
  EnumName extends string,
  EnumValueName extends string
> = `${NameOfEnum<EnumName>}.${NameOf<EnumValueName, "dynamicEnumValue">}`;

type Simplify<T> = { [K in keyof T]: T[K] } & {};

type MustBeDynamic<
  Name extends string,
  Method extends string
> = `'${Name}.${Method}(..)' is only allowed when ${Name} has the @@dynamic block attribute in the .baml file. Did you mean to use .get${Capitalize<Method>}(..) or add @@dynamic to ${Name}?`;

type PreventRedefinition<
  Name extends string,
  ExistingKeys
> = Name extends keyof ExistingKeys
  ? `Class '${Name}' is already statically defined. Use .getClass('${Name}') instead of .class('${Name}') to access the existing class.`
  : Name;

type PreventEnumRedefinition<
  Name extends string,
  ExistingKeys
> = Name extends keyof ExistingKeys
  ? `Enum '${Name}' is already statically defined. Use .getEnum('${Name}') instead of .enum('${Name}') to access the existing enum.`
  : Name;

type PreventPropertyRedefinition<
  PropertyName extends string,
  ExistingProperties,
  ClassName extends string = string
> = PropertyName extends keyof ExistingProperties
  ? `Property '${ClassName}.${PropertyName}' is already statically defined. Use .getProperty('${PropertyName}') instead of .property('${PropertyName}') to access the existing property.`
  : PropertyName;

type PreventEnumValueRedefinition<
  ValueName extends string,
  ExistingValues,
  EnumName extends string = string
> = ValueName extends keyof ExistingValues
  ? `Enum value '${EnumName}.${ValueName}' is already statically defined. Use .getValue('${ValueName}') instead of .value('${ValueName}') to access the existing value.`
  : ValueName;

type ValidatePropertiesConfig<
  Configs extends Record<string, any>,
  ExistingProperties
> = {
  [K in keyof Configs]: K extends keyof ExistingProperties
    ? Configs[K] extends { type: any }
      ? never
      : Configs[K]
    : Configs[K];
};

export type ClassPropertyConfig = {
  alias?: string | null;
  description?: string | null;
  skip?: boolean | null;
};

export type ExistingClassProperty = Partial<Omit<ClassPropertyConfig, "type">>;

export type NewClassProperty =
  | Type
  | ({
      type: Type;
    } & ExistingClassProperty);

export type ExistingEnumValue = {
  alias?: string | null;
  description?: string | null;
  skip?: boolean | null;
};

export type NewEnumValue = string | ({ value: string } & ExistingEnumValue);

export type ClassPropertyRecord = Record<string, ClassPropertyConfig>;

export type EnumValueConfig = {
  value?: string | null;
  alias?: string | null;
  description?: string | null;
  skip?: boolean | null;
};

export type EnumValueRecord = Record<string, EnumValueConfig>;

export type TypeAliasConfig<AliasType extends Type> = {
  type: AliasType;
  alias?: string | null;
  description?: string | null;
};

export type ClassShape<
  Properties extends ClassPropertyRecord,
  Dynamic extends boolean
> = {
  properties: Properties;
  dynamic: Dynamic;
};

export type EnumShape<
  Values extends EnumValueRecord,
  Dynamic extends boolean
> = {
  values: Values;
  dynamic: Dynamic;
};

export type TypeAliasShape = {
  dynamic: false;
};

export type ClassDictionary = {
  [K in string]: ClassShape<ClassPropertyRecord, boolean>;
};

export type EnumDictionary = Record<
  string,
  EnumShape<EnumValueRecord, boolean>
>;

export type TypeAliasDictionary = Record<string, TypeAliasShape>;

type PropertyType<
  Properties extends ClassPropertyRecord,
  PropertyName extends string
> = Type;

type ApplyClassProperty<
  Properties extends ClassPropertyRecord,
  PropertyName extends string,
  Config extends NewClassProperty | ExistingClassProperty
> = Config extends ExistingClassProperty
  ? Simplify<
      Properties & {
        [K in PropertyName]: Simplify<
          (K extends keyof Properties ? Properties[K] : ClassPropertyConfig) &
            Config
        >;
      }
    >
  : Simplify<
      Properties & {
        [K in PropertyName]: Config & NewClassProperty;
      }
    >;

type ApplyClassProperties<
  Properties extends ClassPropertyRecord,
  Configs extends Record<string, NewClassProperty | ExistingClassProperty>
> = Simplify<
  Properties & {
    [K in keyof Configs & string]: Configs[K] extends ExistingClassProperty
      ? Simplify<
          (K extends keyof Properties ? Properties[K] : ClassPropertyConfig) &
            Configs[K]
        >
      : Configs[K] & NewClassProperty;
  }
>;

type EnumValueLiteral<Value extends NewEnumValue> = Value extends string
  ? Value
  : "dynamicEnumValue";

type ApplyEnumValue<
  Values extends EnumValueRecord,
  ValueName extends string,
  Value extends NewEnumValue | ExistingEnumValue
> = Value extends ExistingEnumValue
  ? Simplify<
      Values & {
        [K in ValueName]: Simplify<
          (K extends keyof Values ? Values[K] : { value: K }) & Value
        >;
      }
    >
  : Simplify<
      Values & {
        [K in ValueName]: Value & NewEnumValue;
      }
    >;

type ApplyEnumValues<
  Values extends EnumValueRecord,
  Configs extends Record<string, NewEnumValue | ExistingEnumValue>
> = Simplify<
  Values & {
    [K in keyof Configs & string]: Configs[K] extends ExistingEnumValue
      ? Simplify<
          (K extends keyof Values ? Values[K] : { value: K }) & Configs[K]
        >
      : Configs[K] & NewEnumValue;
  }
>;

type InferClassProperties<
  Classes,
  Name extends string
> = Name extends keyof Classes
  ? Classes[Name] extends ClassShape<infer Props, any>
    ? Props
    : ClassPropertyRecord
  : ClassPropertyRecord;

type InferClassDynamic<
  Classes,
  Name extends string
> = Name extends keyof Classes
  ? Classes[Name] extends ClassShape<any, infer Dynamic>
    ? Dynamic
    : boolean
  : true;

type InferClassType<Classes, Name extends string> = Name extends keyof Classes
  ? Classes[Name] extends ClassShape<any, infer ClassType>
    ? ClassType
    : Type
  : Type;

type InferEnumValues<Enums, Name extends string> = Name extends keyof Enums
  ? Enums[Name] extends EnumShape<infer Values, any>
    ? Values
    : EnumValueRecord
  : EnumValueRecord;

type InferEnumDynamic<Enums, Name extends string> = Name extends keyof Enums
  ? Enums[Name] extends EnumShape<any, infer Dynamic>
    ? Dynamic
    : boolean
  : true;

type InferEnumType<Enums, Name extends string> = Name extends keyof Enums
  ? Enums[Name] extends EnumShape<any, infer EnumType>
    ? EnumType
    : Type
  : Type;

type InferAliasType<Aliases, Name extends string> = Type;

type InferAliasDynamic<Aliases, Name extends string> = false;

export interface WithAliasAndDescription<
  Dynamic extends boolean,
  Name extends string
> {
  alias: Dynamic extends true
    ? (new_alias: string | null) => this
    : MustBeDynamic<Name, "alias">;
  description: Dynamic extends true
    ? (new_description: string | null) => this
    : MustBeDynamic<Name, "description">;

  getAlias(): string | null;
  getDescription(): string | null;
}

export interface PropertyBuilder<
  ClassName extends string,
  PropertyName extends string,
  Dynamic extends boolean
> extends WithAliasAndDescription<
    Dynamic,
    NameOfClassProperty<ClassName, PropertyName>
  > {
  getClassName(): NameOfClass<ClassName, string>;
  getName(): NameOfProperty<PropertyName, string>;
  getType(): Type;
}

export interface ClassBuilder<
  ClassName extends string,
  Properties extends ClassPropertyRecord,
  Dynamic extends boolean
> extends WithAliasAndDescription<Dynamic, NameOfClass<ClassName>> {
  getType(): Type;
  getName(): NameOfClass<ClassName, string>;

  getProperties(): readonly PropertyBuilder<
    ClassName,
    keyof Properties & string,
    Dynamic
  >[];

  getProperty<PropertyKey extends keyof Properties & string>(
    property_name: PropertyKey
  ): PropertyBuilder<ClassName, PropertyKey, Dynamic>;

  getProperty<PropertyKey extends string>(
    property_name: PropertyKey
  ): PropertyBuilder<ClassName, PropertyKey, Dynamic>;

  property<
    PropertyKey extends string,
    Config extends NewClassProperty | ExistingClassProperty
  >(
    this: Dynamic extends true
      ? ClassBuilder<ClassName, Properties, Dynamic>
      : MustBeDynamic<ClassName, "property">,
    property_name: PreventPropertyRedefinition<
      PropertyKey,
      Properties,
      ClassName
    >,
    config: Config
  ): PropertyBuilder<ClassName, PropertyKey, Dynamic>;

  properties<
    Configs extends Record<string, NewClassProperty | ExistingClassProperty>
  >(
    this: Dynamic extends true
      ? ClassBuilder<ClassName, Properties, Dynamic>
      : MustBeDynamic<ClassName, "properties">,
    configs: Configs &
      (keyof Configs & keyof Properties extends never
        ? {}
        : {
            [K in keyof Configs &
              keyof Properties]: `Property '${ClassName}.${K &
              string}' is already statically defined. Remove the 'type' field - use only: { alias?, description? }`;
          })
  ): ClassBuilder<
    ClassName,
    ApplyClassProperties<Properties, Configs>,
    Dynamic
  >;
}

export interface EnumValueBuilder<
  EnumName extends string,
  ValueName extends string,
  Dynamic extends boolean
> extends WithAliasAndDescription<
    Dynamic,
    NameOfEnumValue<EnumName, ValueName>
  > {
  getEnumName(): NameOfEnum<EnumName>;
  getName(): NameOfEnumValue<EnumName, ValueName>;
  getType(): Type;

  getSkip(): boolean;
  skip(): this;
}

export interface EnumBuilder<
  EnumName extends string,
  Values extends EnumValueRecord,
  Dynamic extends boolean,
  EnumType extends Type = Type
> extends WithAliasAndDescription<Dynamic, NameOfEnum<EnumName>> {
  readonly enumName: NameOfEnum<EnumName>;
  readonly type: EnumType;

  listValues(): readonly EnumValueBuilder<
    EnumName,
    keyof Values & string,
    Dynamic
  >[];

  getValue<ValueKey extends keyof Values & string>(
    value_name: ValueKey
  ): EnumValueBuilder<EnumName, ValueKey, Dynamic>;

  getValue<ValueKey extends string>(
    value_name: ValueKey
  ): EnumValueBuilder<EnumName, ValueKey, Dynamic>;

  value<ValueConfig extends NewEnumValue>(
    this: Dynamic extends true
      ? EnumBuilder<EnumName, Values, Dynamic, EnumType>
      : MustBeDynamic<EnumName, "value">,
    value_name: PreventEnumValueRedefinition<
      EnumValueLiteral<ValueConfig>,
      Values,
      EnumName
    >
  ): EnumValueBuilder<EnumName, EnumValueLiteral<ValueConfig>, Dynamic>;

  values<NewValues extends readonly string[]>(
    this: Dynamic extends true
      ? EnumBuilder<EnumName, Values, Dynamic, EnumType>
      : MustBeDynamic<EnumName, "values">,
    values: NewValues extends readonly (infer V)[]
      ? V extends keyof Values
        ? `Enum value '${EnumName}.${V &
            string}' is already statically defined. Use getValue('${V &
            string}') instead.`[]
        : NewValues
      : never
  ): EnumBuilder<
    EnumName,
    Values & { [K in NewValues[number]]: { value: K } },
    Dynamic,
    EnumType
  >;
}

type ClassDictionaryToBuilder<Classes extends ClassDictionary> = {
  [K in keyof Classes & string]: ClassBuilder<
    K,
    Classes[K] extends ClassShape<infer Props, infer Dynamic>
      ? Props
      : ClassPropertyRecord,
    Classes[K] extends ClassShape<infer Props, infer Dynamic>
      ? Dynamic
      : boolean
  >;
};

export interface TypeRegistry<
  Classes extends ClassDictionary = {},
  Enums extends EnumDictionary = {},
  TypeAliases extends TypeAliasDictionary = {}
> {
  listClasses(): readonly ClassBuilder<string, {}, true>[];
  listEnums(): readonly EnumBuilder<string, {}, true>[];

  getClass<Name extends keyof Classes & string>(
    class_name: Name
  ): ClassBuilder<
    Name,
    InferClassProperties<Classes, Name>,
    InferClassDynamic<Classes, Name>
  >;

  getClass<Name extends string>(
    class_name: Name
  ): ClassBuilder<Name, ClassPropertyRecord, true>;

  getEnum<Name extends keyof Enums & string>(
    enum_name: Name
  ): EnumBuilder<
    Name,
    InferEnumValues<Enums, Name>,
    InferEnumDynamic<Enums, Name>
  >;

  getEnum<Name extends string>(
    enum_name: Name
  ): EnumBuilder<Name, EnumValueRecord, true>;

  class<
    Name extends string,
    ExistingProps extends ClassPropertyRecord = {},
    Props extends Record<string, NewClassProperty> = {}
  >(
    class_name: PreventRedefinition<Name, Classes>,
    properties?: ExistingProps & Props
  ): ClassBuilder<Name, ExistingProps & Props, true>;

  enum<
    Name extends string,
    Values extends EnumValueRecord = {},
    ExistingValues extends EnumValueRecord = {}
  >(
    enum_name: PreventEnumRedefinition<Name, Enums>,
    values?: Values
  ): EnumBuilder<Name, ExistingValues & Values, true>;

  toString(): string;

  string(): Type;
  literalString<Value extends string>(value: Value): Type;
  literalInt<Value extends number>(value: Value): Type;
  literalBool<Value extends boolean>(value: Value): Type;
  int(): Type;
  float(): Type;
  bool(): Type;
  list<ItemType extends Type>(type: ItemType): Type;
  null(): Type;
  map<KeyType extends Type, ValueType extends Type>(
    keyType: KeyType,
    valueType: ValueType
  ): Type;
  union<Types extends readonly Type[]>(types: Types): Type;

  getDependencies(type: Type): Type[];
}

function test(
  tb: TypeRegistry<
    {
      MyClass: {
        properties: {
          my_property: {};
        };
        dynamic: false;
      };
      MyClass2: {
        properties: {
          my_property: {};
        };
        dynamic: true;
      };
    },
    {
      MyEnum: {
        values: {
          value1: {};
        };
        dynamic: false;
      };
    }
  >
) {
  tb.class("Class").property("property", tb``.string());
  tb.getClass("MyClass").getProperty("property").getType();

  tb.enum("Enum").value("value");

  tb.listClasses().map((cls) => {
    cls.getName();
  });

  tb.class("MyClass").property("property", tb.string());
  tb.enum("MyEnum").value("newValue");
  tb.getClass("MyClass").property("my_property", tb.string());
  tb.getClass("MyClass2").properties({
    my_property: { type: tb.string(), alias: "myProp" },
    new_property: tb.int(),
    another_property: { type: tb.string() },
  });
  tb.getEnum("MyEnum").value("value1");
  tb.getEnum("MyEnum").values(["value1", "value2", "value3"]);
}
m;
