RUSTC := rustc
BUILD := build
LIB := $(BUILD)/$(shell $(RUSTC) --crate-file-name src/toml.rs)
TEST := $(BUILD)/tomltest

all: $(LIB)

-include $(BUILD)/toml.d
-include $(BUILD)/tomltest.d

$(LIB): src/toml.rs
	@mkdir -p $(@D)
	$(RUSTC) -O $< --out-dir $(@D) --dep-info

check: $(TEST)
	$(TEST)

$(TEST): src/toml.rs
	$(RUSTC) $< --test -o $@ --dep-info

clean:
	rm -rf $(BUILD)
