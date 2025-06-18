
# tools/dump_ffi_api.rb
require_relative "../lib/baml"
require "pp"

def list_under(ns)
  ns.constants(false).map do |const_name|
    const = ns.const_get(const_name)
    next unless const.is_a?(Module) || const.is_a?(Class)
    {
      const: "#{ns}::#{const_name}",
      methods: const.instance_methods(false).sort,
      cmethods: const.singleton_methods(false).sort,
    }
  end.compact
end

pp list_under(Baml::Ffi)
