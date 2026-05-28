# ============================================================
# Darvium Plugin — Claude Code プラグイン管理
# この Makefile は crates/darvium をカレントディレクトリとした
# ときに make install-darvium / make update-darvium を提供する。
# crates/darvium/.claude/settings.json に project scope で設定され、
# Zed でこのディレクトリをルートとして開いたときのみ有効となる。
# ============================================================

install-darvium:
ifeq ($(OS),Windows_NT)
	npx @anthropic-ai/claude-code plugin marketplace list 2>&1 | findstr "darvium-marketplace" >nul || \
		npx @anthropic-ai/claude-code plugin marketplace add t-kawata/darvium-plugin
	npx @anthropic-ai/claude-code plugin install darvium@darvium-marketplace --scope project
else
	@if ! claude plugin marketplace list 2>&1 | grep -q "darvium-marketplace"; then \
		echo "Adding darvium marketplace..."; \
		claude plugin marketplace add t-kawata/darvium-plugin; \
	fi
	claude plugin install darvium@darvium-marketplace --scope project
endif
	@echo "Darvium plugin installed (project scope)."

update-darvium:
ifeq ($(OS),Windows_NT)
	npx @anthropic-ai/claude-code plugin marketplace update darvium-marketplace
	npx @anthropic-ai/claude-code plugin install darvium@darvium-marketplace --scope project
	node scripts/plugin/sync-plugin.cjs
else
	claude plugin marketplace update darvium-marketplace
	claude plugin install darvium@darvium-marketplace --scope project
	node scripts/plugin/sync-plugin.cjs
endif
	@echo "Darvium plugin updated."

# push / pull / commit はルートプロジェクト(~h/shyme/mycute)に委譲
push:
	cd ~/shyme/mycute && PUSH_MSG="$(msg)" $(MAKE) push

commit:
	cd ~/shyme/mycute && PUSH_MSG="$(msg)" $(MAKE) commit

pull:
	cd ~/shyme/mycute && $(MAKE) pull

run-observation:
	cargo run -p darvium --features server --bin darvium-observer -- --port 3333
