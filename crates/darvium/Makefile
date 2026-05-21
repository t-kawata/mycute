# ============================================================
# Darvium Plugin — Claude Code プラグイン管理
# この Makefile は crates/darvium をカレントディレクトリとした
# ときに make install-darvium / make update-darvium を提供する。
# crates/darvium/.claude/settings.json に project scope で設定され、
# Zed でこのディレクトリをルートとして開いたときのみ有効となる。
# ============================================================

install-darvium:
	@if ! claude plugin marketplace list 2>&1 | grep -q "darvium-marketplace"; then \
		echo "Adding darvium marketplace..."; \
		claude plugin marketplace add t-kawata/darvium-plugin; \
	fi
	claude plugin install darvium@darvium-marketplace --scope project
	@echo "Darvium plugin installed (project scope)."

update-darvium:
	claude plugin marketplace update darvium-marketplace
	claude plugin install darvium@darvium-marketplace --scope project
	@echo "Darvium plugin updated."

# push / pull はルートプロジェクト(~h/shyme/mycute)に委譲
push:
	cd ~/shyme/mycute && $(MAKE) push

pull:
	cd ~/shyme/mycute && $(MAKE) pull
