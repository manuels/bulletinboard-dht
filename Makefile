NAME=bulletinboard
VERSION=$(shell grep version Cargo.toml | head -n1 | cut -d \" -f 2)

.PHONY: package
package:
	cargo build --release
	for FORMAT in deb rpm; do \
		fpm -s dir -t $$FORMAT -n $(NAME) -v $(VERSION) \
		  	target/release/bulletinboard=/usr/bin/ \
		  	org.manuel.BulletinBoard.service=/usr/share/dbus-1/services/ ; \
	done

