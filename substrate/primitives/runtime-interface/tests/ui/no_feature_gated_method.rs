// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Due to https://github.com/dtolnay/trybuild/issues/302, the `substrate_runtime`
// cfg is also unexpected. This is a `trybuild` bug, and this part of error is not
// exposed to a real developer.

use sp_runtime_interface::runtime_interface;

#[runtime_interface]
trait Test {
	fn foo() {}

	#[cfg(feature = "bar-feature")]
	fn bar() {}

	#[cfg(not(feature = "bar-feature"))]
	fn qux() {}
}

fn main() {
	test::foo();
	test::bar();
	test::qux();
}
