# Copyright (c) 2024 Elektrobit Automotive GmbH
#
# This program and the accompanying materials are made available under the
# terms of the Apache License, Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0.
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
# WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
# License for the specific language governing permissions and limitations
# under the License.
#
# SPDX-License-Identifier: Apache-2.0

FROM ghcr.io/eclipse-ankaios/devcontainer-base:0.10.4

ARG USERNAME=vscode

# add some convenience aliases
RUN echo 'alias ll="ls -la"' | tee -a /home/${USERNAME}/.bashrc /home/${USERNAME}/.zshrc
RUN echo 'alias ..="cd .."' | tee -a /home/${USERNAME}/.bashrc /home/${USERNAME}/.zshrc
RUN echo 'alias ...="cd ../.."' | tee -a /home/${USERNAME}/.bashrc /home/${USERNAME}/.zshrc

USER ${USERNAME}
