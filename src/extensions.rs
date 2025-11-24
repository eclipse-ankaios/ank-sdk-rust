// Copyright (c) 2025 Elektrobit Automotive GmbH
//
// This program and the accompanying materials are made available under the
// terms of the Apache License, Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0.
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations
// under the License.
//
// SPDX-License-Identifier: Apache-2.0

//! This module contains extensions to the standard library
//! that are used throughout the project.

use std::fmt::Display;

/// Trait that provides a method to unwrap an `Option<T>` for cases where
/// the `Option` is expected to always contain a value.
pub trait UnreachableOption<T> {
    /// Returns the contained [`Some`] value or panics
    /// by executing the unreachable! macro.
    ///
    /// # Examples
    ///
    /// ```rust,should_panic
    /// use ankaios_sdk::extensions::UnreachableOption;
    /// assert_eq!(Some::<&str>("foo").unwrap_or_unreachable(), "foo");
    ///
    /// // shall panic because unreachable is hit
    /// None::<&str>.unwrap_or_unreachable();
    /// ```
    fn unwrap_or_unreachable(self) -> T;
}

impl<T> UnreachableOption<T> for Option<T> {
    fn unwrap_or_unreachable(self) -> T {
        match self {
            Some(value) => value,
            None => std::unreachable!(),
        }
    }
}

/// Trait that provides a method to unwrap a `Result<T, E>` for cases where
/// the `Result` is expected to always contain an Ok.
pub trait UnreachableResult<T, E> {
    /// Returns the contained [`Ok`] value or panics
    /// by executing the unreachable! macro with logging the error E of the Result<T,E>.
    ///
    /// # Examples
    ///
    /// ```rust,should_panic
    /// use ankaios_sdk::extensions::UnreachableResult;
    /// assert_eq!(Ok::<&str, &str>("foo").unwrap_or_unreachable(), "foo");
    ///
    /// // shall panic because unreachable is hit
    /// Err::<&str, &str>("some error").unwrap_or_unreachable();
    /// ```
    fn unwrap_or_unreachable(self) -> T;
}

impl<T, E: Display> UnreachableResult<T, E> for Result<T, E> {
    fn unwrap_or_unreachable(self) -> T {
        match self {
            Ok(value) => value,
            Err(error) => {
                std::unreachable!("{error}")
            }
        }
    }
}

//////////////////////////////////////////////////////////////////////////////
//                 ########  #######    #########  #########                //
//                    ##     ##        ##             ##                    //
//                    ##     #####     #########      ##                    //
//                    ##     ##                ##     ##                    //
//                    ##     #######   #########      ##                    //
//////////////////////////////////////////////////////////////////////////////

mod tests {
    #[allow(unused_imports)]
    use super::{UnreachableOption, UnreachableResult};

    #[test]
    #[should_panic(expected = "internal error: entered unreachable code")]
    fn test_unreachable_case() {
        let _ = None::<&str>.unwrap_or_unreachable();
    }

    #[test]
    #[should_panic(expected = "internal error: entered unreachable code: test error")]
    fn test_unreachable_result_case() {
        let _ = Err::<&str, &str>("test error").unwrap_or_unreachable();
    }
}
