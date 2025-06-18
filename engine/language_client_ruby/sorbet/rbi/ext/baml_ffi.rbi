# typed: strict
# DO NOT EDIT BY CODE-GEN AFTER THIS POINT — curate manually.

module Baml
    module Ffi
      # -----------------------------------------------------------------------
      # Builders
      # -----------------------------------------------------------------------
      class EnumBuilder
        sig { params(name: T.untyped).void }
        def alias(name); end
  
        sig { params(field_name: T.untyped).void }
        def field(field_name); end
  
        sig { params(value: T.untyped).void }
        def value(value); end
      end
  
      class ClassBuilder
        sig { params(field_name: T.untyped).void }
        def field(field_name); end
  
        sig { params(prop_name: T.untyped).void }
        def property(prop_name); end
      end
  
      class EnumValueBuilder
        sig { params(name: T.untyped).void }
        def alias(name); end
  
        sig { params(text: T.untyped).void }
        def description(text); end
  
        sig { void }
        def skip; end
      end
  
      class ClassPropertyBuilder
        sig { params(name: T.untyped).void }
        def alias(name); end
  
        sig { params(text: T.untyped).void }
        def description(text); end
  
        sig { params(type: FieldType).void }
        def type(type); end
      end
  
      class FieldType
        sig { void }
        def list; end
  
        sig { void }
        def optional; end
      end
  
      # -----------------------------------------------------------------------
      # Runtime logging / usage structs
      # -----------------------------------------------------------------------
      class Timing
        sig { returns(Integer) }
        def duration_ms; end
  
        sig { returns(Integer) }
        def start_time_utc_ms; end
  
        sig { returns(Integer) }
        def time_to_first_parsed_ms; end
  
        sig { returns(String) }
        def to_s; end
      end
  
      class StreamTiming
        sig { returns(Integer) }
        def duration_ms; end
  
        sig { returns(Integer) }
        def start_time_utc_ms; end
  
        sig { returns(Integer) }
        def time_to_first_parsed_ms; end
  
        sig { returns(Integer) }
        def time_to_first_token_ms; end
  
        sig { returns(String) }
        def to_s; end
      end
  
      class Usage
        sig { returns(Integer) }
        def input_tokens; end
  
        sig { returns(Integer) }
        def output_tokens; end
  
        sig { returns(String) }
        def to_s; end
      end
  
      class HTTPRequest
        sig { returns(String) }
        def body; end
        sig { returns(T::Hash[String, T.untyped]) }
        def headers; end
        sig { returns(String) }
        def id; end
        sig { returns(String) }
        def method; end
        sig { returns(String) }
        def to_s; end
        sig { returns(String) }
        def url; end
      end
  
      class HTTPResponse
        sig { returns(String) }
        def body; end
        sig { returns(T::Hash[String, T.untyped]) }
        def headers; end
        sig { returns(Integer) }
        def status; end
        sig { returns(String) }
        def to_s; end
      end
  
      class HTTPBody
        sig { returns(String) }
        def json; end
        sig { returns(String) }
        def raw; end
        sig { returns(String) }
        def text; end
      end
  
      # -----------------------------------------------------------------------
      # Media helpers
      # -----------------------------------------------------------------------
      module Audio
        sig { params(b64: String).returns(Audio) }
        def self.from_base64(b64); end
  
        sig { params(url: String).returns(Audio) }
        def self.from_url(url); end
      end
  
      module Image
        sig { params(b64: String).returns(Image) }
        def self.from_base64(b64); end
  
        sig { params(url: String).returns(Image) }
        def self.from_url(url); end
      end
  
      # -----------------------------------------------------------------------
      # Core engine types
      # -----------------------------------------------------------------------
      class FunctionLog
        sig { returns(T::Array[LLMCall]) }
        def calls; end
        sig { returns(String) }
        def function_name; end
        sig { returns(String) }
        def id; end
        sig { returns(String) }
        def log_type; end
        sig { returns(String) }
        def raw_llm_response; end
        sig { returns(LLMCall) }
        def selected_call; end
        sig { returns(Timing) }
        def timing; end
        sig { returns(String) }
        def to_s; end
        sig { returns(Usage) }
        def usage; end
      end
  
      class LLMCall
        sig { returns(String) }
        def client_name; end
        sig { returns(HTTPRequest) }
        def http_request; end
        sig { returns(HTTPResponse) }
        def http_response; end
        sig { returns(String) }
        def provider; end
        sig { returns(T::Boolean) }
        def selected; end
        sig { returns(Timing) }
        def timing; end
        sig { returns(String) }
        def to_s; end
        sig { returns(Usage) }
        def usage; end
      end
  
      class LLMStreamCall < LLMCall; end
  
      class FunctionResult
        sig { returns(T::Boolean) }
        def parsed_using_types; end
      end
  
      class FunctionResultStream
        sig { returns(T::Boolean) }
        def done; end
      end
  
      # -----------------------------------------------------------------------
      # Builders for creating types at runtime
      # -----------------------------------------------------------------------
      class TypeBuilder
        # NOTE: return types here guessed — replace with real classes you expose.
        sig { params(arg: T.untyped).returns(TypeBuilder) }
        def add_baml(arg); end
  
        sig { returns(TypeBuilder) }
        def bool; end
  
        sig { params(name: String).returns(ClassBuilder) }
        def class_(name); end
  
        sig { params(name: String).returns(EnumBuilder) }
        def enum(name); end
  
        sig { returns(TypeBuilder) }
        def float; end
        sig { returns(TypeBuilder) }
        def int; end
        sig { params(inner: T.untyped).returns(TypeBuilder) }
        def list(inner); end
  
        sig { params(val: T::Boolean).returns(TypeBuilder) }
        def literal_bool(val); end
        sig { params(val: Integer).returns(TypeBuilder) }
        def literal_int(val); end
        sig { params(val: String).returns(TypeBuilder) }
        def literal_string(val); end
  
        sig { params(key_type: T.untyped, value_type: T.untyped).returns(TypeBuilder) }
        def map(key_type, value_type); end
  
        sig { returns(TypeBuilder) }
        def null; end
        sig { returns(TypeBuilder) }
        def optional; end
        sig { returns(TypeBuilder) }
        def string; end
        sig { returns(String) }
        def to_s; end
  
        sig { params(types: T::Array[T.untyped]).returns(TypeBuilder) }
        def union(types); end
      end
  
      # -----------------------------------------------------------------------
      # Glue / runtime
      # -----------------------------------------------------------------------
      class ClientRegistry
        sig { params(client: T.untyped).void }
        def add_llm_client(client); end
  
        sig { params(client_name: String).void }
        def set_primary(client_name); end
      end
  
      class Collector
        sig { params(id: String).void }
        def initialize(id); end
  
        sig { returns(String) }
        def id; end
        sig { returns(FunctionLog) }
        def last; end
        sig { returns(T::Array[FunctionLog]) }
        def logs; end
        sig { returns(String) }
        def to_s; end
        sig { returns(Usage) }
        def usage; end
  
        # Low-level debug helpers
        sig { returns(Integer) }
        def self.__function_call_count; end
        sig { void }
        def self.__print_storage; end
      end
  
      class RuntimeContextManager; end
  
      class BamlRuntime
        sig { params(dir: String).returns(BamlRuntime) }
        def self.from_directory(dir); end
        sig { params(files: T::Array[String]).returns(BamlRuntime) }
        def self.from_files(files); end
  
        sig { params(name: String, args: T.untyped).returns(HTTPRequest) }
        def build_request(name, args); end
  
        sig { params(name: String, args: T.untyped).returns(FunctionResult) }
        def call_function(name, args); end
  
        sig { returns(RuntimeContextManager) }
        def create_context_manager; end
  
        sig { params(name: String, llm_response: String).returns(FunctionResult) }
        def parse_llm_response(name, llm_response); end
  
        sig { params(name: String, args: T.untyped).returns(FunctionResultStream) }
        def stream_function(name, args); end
      end
    end
  end
  