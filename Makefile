RUSTC ?= rustc
RUSTDOC ?= rustdoc
BUILD ?= build
LIB := $(BUILD)/$(shell $(RUSTC) --crate-file-name src/toml.rs)
TEST := $(BUILD)/tomltest

all: $(LIB)

-include $(BUILD)/toml.d
-include $(BUILD)/tomltest.d

$(LIB): src/toml.rs
	@mkdir -p $(@D)
	$(RUSTC) $< --out-dir $(@D) --dep-info

check: $(TEST) doctest
	$(TEST)

$(TEST): src/toml.rs
	$(RUSTC) $< --test -o $@ --dep-info

doctest: $(LIB)
	$(RUSTDOC) --test -L $(BUILD) src/toml.rs

clean:
	rm -rf $(BUILD)
