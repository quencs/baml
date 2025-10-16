import { ClassBuilder as _ClassBuilder, EnumBuilder as _EnumBuilder, ClassPropertyBuilder as _ClassPropertyBuilder, EnumValueBuilder as _EnumValueBuilder, FieldType, TypeBuilder as _TypeBuilder, BamlRuntime } from "../native";
type IfDynamic<D extends boolean, T, F = never> = D extends true ? T : F;
type MustBeDynamic<Name extends string, Method extends string> = `'${Name}.${Method}' is only allowed when ${Name} is marked @@dynamic.`;
export declare class TypeBuilder {
    private tb;
    constructor({ runtime }: {
        runtime: BamlRuntime;
    });
    _tb(): _TypeBuilder;
    reset(): void;
    toString(): string;
    string(): FieldType;
    literalString(value: string): FieldType;
    literalInt(value: number): FieldType;
    literalBool(value: boolean): FieldType;
    int(): FieldType;
    float(): FieldType;
    bool(): FieldType;
    list(type: FieldType): FieldType;
    null(): FieldType;
    map(keyType: FieldType, valueType: FieldType): FieldType;
    union(types: FieldType[]): FieldType;
    addClass(name: string): ClassBuilder<string, string, true>;
    getClass<Name extends string, Properties extends string, Dynamic extends boolean>(name: Name): ClassBuilder<Name, Properties, Dynamic>;
    addEnum(name: string): EnumBuilder<string, string, true>;
    getEnum<Name extends string, Values extends string, Dynamic extends boolean>(name: Name): EnumBuilder<Name, Values, Dynamic>;
    addBaml(baml: string): void;
}
export declare class ClassBuilder<Name extends string, PropertyName extends string, Dynamic extends boolean> {
    private readonly cb;
    constructor(cb: _ClassBuilder);
    type(): FieldType;
    listProperties(): Array<[
        IfDynamic<Dynamic, PropertyName | string, PropertyName>,
        ClassPropertyBuilder<Name, IfDynamic<Dynamic, PropertyName | string, PropertyName>, Dynamic>
    ]>;
    reset(): void;
    getProperty(name: PropertyName): ClassPropertyBuilder<Name, PropertyName, Dynamic>;
    getProperty(name: IfDynamic<Dynamic, string, never>): ClassPropertyBuilder<Name, string, Dynamic>;
    /**
     * addProperty:
     *  - only allowed if Class marked with @@dynamic
     */
    addProperty(this: Dynamic extends true ? ClassBuilder<Name, PropertyName, Dynamic> : MustBeDynamic<Name, "addProperty">, name: string, fieldType: FieldType): ClassPropertyBuilder<Name, string, Dynamic>;
    /**
     * removeProperty:
     *  - only allowed if Class marked with @@dynamic
     */
    removeProperty(this: Dynamic extends true ? ClassBuilder<Name, PropertyName, Dynamic> : MustBeDynamic<Name, "removeProperty">, name: IfDynamic<Dynamic, PropertyName | string, never>): void;
    /**
     * setAlias:
     *  - only allowed if Class marked with @@dynamic
     */
    setAlias(this: Dynamic extends true ? ClassBuilder<Name, PropertyName, Dynamic> : MustBeDynamic<Name, "setAlias">, alias: string | null): ClassBuilder<Name, PropertyName, Dynamic>;
    /**
     * setDescription:
     *  - only allowed if Class marked with @@dynamic
     */
    setDescription(this: Dynamic extends true ? ClassBuilder<Name, PropertyName, Dynamic> : MustBeDynamic<Name, "setDescription">, description: string | null): ClassBuilder<Name, PropertyName, Dynamic>;
    alias(): string | null;
    description(): string | null;
    source(): "baml" | "dynamic";
}
export declare class EnumBuilder<Name extends string, ValueName extends string, Dynamic extends boolean> {
    private readonly eb;
    constructor(eb: _EnumBuilder);
    type(): FieldType;
    listValues(): Array<[
        IfDynamic<Dynamic, ValueName | string, ValueName>,
        EnumValueBuilder<Name, IfDynamic<Dynamic, ValueName | string, ValueName>, Dynamic>
    ]>;
    /**
     * addValue:
     *  - only allowed if Enum marked with @@dynamic
     */
    addValue(this: Dynamic extends true ? EnumBuilder<Name, ValueName, Dynamic> : MustBeDynamic<Name, "addValue">, name: string): EnumValueBuilder<Name, ValueName, Dynamic>;
    getValue(name: ValueName): EnumValueBuilder<Name, ValueName, Dynamic>;
    getValue(name: IfDynamic<Dynamic, string, never>): EnumValueBuilder<Name, string, Dynamic>;
    /**
     * removeValue:
     *  - only allowed if Enum marked with @@dynamic
     */
    removeValue<V extends ValueName>(this: Dynamic extends true ? EnumBuilder<Name, ValueName, Dynamic> : MustBeDynamic<Name, "removeValue">, name: IfDynamic<Dynamic, V, never>): EnumBuilder<Name, ValueName, Dynamic>;
    /**
     * setAlias:
     *  - only allowed if Enum marked with @@dynamic
     */
    setAlias(this: Dynamic extends true ? EnumBuilder<Name, ValueName, Dynamic> : MustBeDynamic<Name, "setAlias">, alias: IfDynamic<Dynamic, string | null, never>): EnumBuilder<Name, ValueName, Dynamic>;
    /**
     * setDescription:
     *  - only allowed if Enum marked with @@dynamic
     */
    setDescription(this: Dynamic extends true ? EnumBuilder<Name, ValueName, Dynamic> : MustBeDynamic<Name, "setDescription">, description: IfDynamic<Dynamic, string | null, never>): EnumBuilder<Name, ValueName, Dynamic>;
    alias(): string | null;
    description(): string | null;
}
declare class ClassPropertyBuilder<ClassName extends string, PropertyName extends string, Dynamic extends boolean> {
    private readonly cpb;
    constructor(cpb: _ClassPropertyBuilder);
    type(): FieldType;
    setType(this: Dynamic extends true ? ClassPropertyBuilder<ClassName, PropertyName, Dynamic> : MustBeDynamic<`${ClassName}.${PropertyName}`, "setType">, fieldType: FieldType): ClassPropertyBuilder<ClassName, PropertyName, Dynamic>;
    setAlias(this: Dynamic extends true ? ClassPropertyBuilder<ClassName, PropertyName, Dynamic> : MustBeDynamic<`${ClassName}.${PropertyName}`, "setAlias">, alias: string | null): ClassPropertyBuilder<ClassName, PropertyName, Dynamic>;
    setDescription(this: Dynamic extends true ? ClassPropertyBuilder<ClassName, PropertyName, Dynamic> : MustBeDynamic<`${ClassName}.${PropertyName}`, "setDescription">, description: string | null): ClassPropertyBuilder<ClassName, PropertyName, Dynamic>;
    alias(): string | null;
    description(): string | null;
    source(): "baml" | "dynamic";
}
export declare class EnumValueBuilder<EnumName extends string, ValueName extends string, Dynamic extends boolean> {
    private readonly evb;
    constructor(evb: _EnumValueBuilder);
    setAlias(this: Dynamic extends true ? EnumValueBuilder<EnumName, ValueName, Dynamic> : MustBeDynamic<`${EnumName}.${ValueName}`, "setAlias">, alias: string | null): EnumValueBuilder<EnumName, ValueName, Dynamic>;
    setDescription(this: Dynamic extends true ? EnumValueBuilder<EnumName, ValueName, Dynamic> : MustBeDynamic<`${EnumName}.${ValueName}`, "setDescription">, description: string | null): EnumValueBuilder<EnumName, ValueName, Dynamic>;
    alias(): string | null;
    description(): string | null;
    source(): "baml" | "dynamic";
}
export {};
//# sourceMappingURL=type_builder.d.ts.map