import typing
from .baml_py import (
    ClassBuilder,
    EnumBuilder,
    FieldType,
    ClassPropertyBuilder,
    EnumValueBuilder,
    TypeBuilder as _TypeBuilder,
    BamlRuntime,
)


class TypeBuilder:
    def __init__(self, runtime: BamlRuntime):
        self.__tb = _TypeBuilder(runtime)
        self.__runtime = runtime

    def reset(self):
        self.__tb.reset()

    def __str__(self) -> str:
        """
        returns a comprehensive string representation of the typebuilder.

        this method provides a detailed view of the entire type hierarchy,
        using the rust implementation to ensure compatibility.

        Format:
            TypeBuilder(
                Classes: [
                    ClassName {
                        property_name type (alias='custom_name', desc='property description'),
                        another_property type (desc='another description'),
                        simple_property type
                    },
                    EmptyClass { }
                ],
                Enums: [
                    EnumName {
                        VALUE (alias='custom_value', desc='value description'),
                        ANOTHER_VALUE (alias='custom'),
                        SIMPLE_VALUE
                    },
                    EmptyEnum { }
                ]
            )

        returns:
            str: the formatted string representation of the typebuilder
        """
        return str(self._tb)

    @property
    def _tb(self) -> _TypeBuilder:
        return self.__tb

    def string(self):
        return self._tb.string()

    def literal_string(self, value: str):
        return self._tb.literal_string(value)

    def literal_int(self, value: int):
        return self._tb.literal_int(value)

    def literal_bool(self, value: bool):
        return self._tb.literal_bool(value)

    def int(self):
        return self._tb.int()

    def float(self):
        return self._tb.float()

    def bool(self):
        return self._tb.bool()

    def list(self, inner: FieldType):
        return self._tb.list(inner)

    def null(self):
        return self._tb.null()

    def map(self, key: FieldType, value: FieldType):
        return self._tb.map(key, value)

    def union(self, types: typing.List[FieldType]):
        return self._tb.union(*types)

    def add_class(self, name: str) -> "NewClassBuilder":
        return NewClassBuilder(self._tb, name)

    def add_enum(self, name: str) -> "NewEnumBuilder":
        return NewEnumBuilder(self._tb, name)

    def add_baml(self, baml: str):
        return self._tb.add_baml(baml)


class NewClassBuilder:
    def __init__(self, tb: _TypeBuilder, name: str):
        self.__bldr = tb.add_class(name)

    def type(self) -> FieldType:
        return self.__bldr.field()

    def list_properties(self) -> typing.List[typing.Tuple[str, ClassPropertyBuilder]]:
        return self.__bldr.list_properties()

    def reset(self):
        self.__bldr.reset()

    def remove_property(self, name: str):
        self.__bldr.remove_property(name)

    def add_property(self, name: str, type: FieldType) -> ClassPropertyBuilder:
        return self.__bldr.add_property(name, type)

    @property
    def class_name(self) -> str:
        return self.__bldr.class_name()


class NewEnumBuilder:
    def __init__(self, tb: _TypeBuilder, name: str):
        self.__bldr = tb.add_enum(name)
        self.__vals = NewEnumValues(self.__bldr)

    def type(self) -> FieldType:
        return self.__bldr.field()

    @property
    def values(self) -> "NewEnumValues":
        return self.__vals

    def list_values(self) -> typing.List[typing.Tuple[str, EnumValueBuilder]]:
        return self.__bldr.list_values()

    def add_value(self, name: str) -> "EnumValueBuilder":
        return self.__bldr.add_value(name)


class NewEnumValues:
    def __init__(self, enum_bldr: EnumBuilder):
        self.__bldr = enum_bldr

    def __getattr__(self, name: str) -> "EnumValueBuilder":
        return self.__bldr.get_value(name)


class EnumValueViewer:
    def __init__(self, bldr: "EnumValueBuilder"):
        self.__bldr = bldr
