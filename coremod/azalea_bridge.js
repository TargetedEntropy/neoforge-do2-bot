var ASMAPI = Java.type('net.neoforged.coremod.api.ASMAPI');
var Opcodes = Java.type('org.objectweb.asm.Opcodes');
var InsnNode = Java.type('org.objectweb.asm.tree.InsnNode');
var InsnList = Java.type('org.objectweb.asm.tree.InsnList');
var TypeInsnNode = Java.type('org.objectweb.asm.tree.TypeInsnNode');
var MethodInsnNode = Java.type('org.objectweb.asm.tree.MethodInsnNode');
var JumpInsnNode = Java.type('org.objectweb.asm.tree.JumpInsnNode');
var LabelNode = Java.type('org.objectweb.asm.tree.LabelNode');
var Label = Java.type('org.objectweb.asm.Label');

/**
 * Azalea Bridge Coremod — allows non-NeoForge clients to connect.
 *
 * Three patches:
 *  1. NetworkComponentNegotiator.negotiate() → always return success
 *  2. ConfigurationInitialization.configureModdedClient() → skip problematic tasks
 *  3. ConfigurationInitialization.configureEarlyTasks() → skip registry sync
 *  4. NetworkRegistry.checkPacket() → suppress UnsupportedOperationException
 */
function initializeCoreMod() {
    ASMAPI.log('INFO', '[AzaleaBridge] v2.5.0 loaded');
    return {
        // --- Patch 1: Succeed negotiation for incompatible clients ---
        // Keeps original logic for real NeoForge clients (so they get proper
        // channel negotiation). Only overrides the result for non-NeoForge
        // clients where negotiation would normally fail.
        'negotiate': {
            'target': {
                'type': 'METHOD',
                'class': 'net.neoforged.neoforge.network.negotiation.NetworkComponentNegotiator',
                'methodName': 'negotiate',
                'methodDesc': '(Ljava/util/List;Ljava/util/List;)Lnet/neoforged/neoforge/network/negotiation/NegotiationResult;'
            },
            'transformer': function(method) {
                ASMAPI.log('INFO', '[AzaleaBridge] Patching NetworkComponentNegotiator.negotiate()');

                // Strategy: before each ARETURN, insert a check:
                //   if (!result.success()) { return new NegotiationResult(List.of(), true, Map.of()); }
                // This preserves the original result for real clients.
                var insns = method.instructions;
                var returnNodes = [];
                for (var i = 0; i < insns.size(); i++) {
                    if (insns.get(i).getOpcode() === Opcodes.ARETURN) {
                        returnNodes.push(insns.get(i));
                    }
                }

                for (var r = 0; r < returnNodes.length; r++) {
                    var retNode = returnNodes[r];
                    // Before ARETURN, stack has: [NegotiationResult]
                    // We insert: DUP, INVOKEVIRTUAL success(), IFNE skip, POP, <create fake>, skip: ARETURN
                    var skipLabel = new LabelNode(new Label());

                    var patch = new InsnList();
                    patch.add(new InsnNode(Opcodes.DUP)); // [result, result]
                    patch.add(new MethodInsnNode(Opcodes.INVOKEVIRTUAL,
                        'net/neoforged/neoforge/network/negotiation/NegotiationResult',
                        'success', '()Z', false));
                    patch.add(new JumpInsnNode(Opcodes.IFNE, skipLabel)); // if true, skip to return

                    // Negotiation failed — replace with empty success
                    patch.add(new InsnNode(Opcodes.POP)); // remove original result
                    patch.add(new TypeInsnNode(Opcodes.NEW,
                        'net/neoforged/neoforge/network/negotiation/NegotiationResult'));
                    patch.add(new InsnNode(Opcodes.DUP));
                    patch.add(new MethodInsnNode(Opcodes.INVOKESTATIC,
                        'java/util/List', 'of', '()Ljava/util/List;', true));
                    patch.add(new InsnNode(Opcodes.ICONST_1)); // true
                    patch.add(new MethodInsnNode(Opcodes.INVOKESTATIC,
                        'java/util/Map', 'of', '()Ljava/util/Map;', true));
                    patch.add(new MethodInsnNode(Opcodes.INVOKESPECIAL,
                        'net/neoforged/neoforge/network/negotiation/NegotiationResult',
                        '<init>', '(Ljava/util/List;ZLjava/util/Map;)V', false));

                    patch.add(skipLabel);
                    // ARETURN follows naturally

                    insns.insertBefore(retNode, patch);
                }

                ASMAPI.log('INFO', '[AzaleaBridge] negotiate() patched: ' + returnNodes.length + ' return point(s), failed results overridden with success');
                return method;
            }
        },

        // --- Patch 2: Skip problematic modded config tasks ---
        // Remove only RegistryDataMapNegotiation, CheckExtensibleEnums, and
        // CheckFeatureFlags registrations. Keep CommonVersionTask, CommonRegisterTask,
        // and SyncConfig so real NeoForge clients still work.
        'configureModdedClient': {
            'target': {
                'type': 'METHOD',
                'class': 'net.neoforged.neoforge.network.ConfigurationInitialization',
                'methodName': 'configureModdedClient',
                'methodDesc': '(Lnet/neoforged/neoforge/network/event/RegisterConfigurationTasksEvent;)V'
            },
            'transformer': function(method) {
                ASMAPI.log('INFO', '[AzaleaBridge] Patching ConfigurationInitialization.configureModdedClient()');

                var badClasses = [
                    'net/neoforged/neoforge/network/configuration/RegistryDataMapNegotiation',
                    'net/neoforged/neoforge/network/configuration/CheckExtensibleEnums',
                    'net/neoforged/neoforge/network/configuration/CheckFeatureFlags'
                ];

                var insns = method.instructions;
                var toRemove = [];
                var removed = 0;

                for (var i = 0; i < insns.size(); i++) {
                    var insn = insns.get(i);
                    if (insn.getOpcode() !== Opcodes.NEW) continue;
                    if (!(insn instanceof TypeInsnNode)) continue;

                    var isBad = false;
                    for (var c = 0; c < badClasses.length; c++) {
                        if (insn.desc === badClasses[c]) { isBad = true; break; }
                    }
                    if (!isBad) continue;

                    ASMAPI.log('INFO', '[AzaleaBridge] Removing task: ' + insn.desc);

                    // Walk backward past labels/line numbers to find the ALOAD
                    // that pushes 'event' for the register() call.
                    var start = i;
                    for (var b = i - 1; b >= 0; b--) {
                        var prev = insns.get(b);
                        if (prev.getOpcode() === -1) continue; // skip pseudo-insns
                        if (prev.getOpcode() === Opcodes.ALOAD) {
                            start = b;
                        }
                        break;
                    }

                    // Walk forward to find the INVOKEVIRTUAL register() call.
                    var end = i;
                    for (var f = i + 1; f < insns.size(); f++) {
                        var fwd = insns.get(f);
                        if (fwd instanceof MethodInsnNode && fwd.name === 'register') {
                            end = f;
                            break;
                        }
                    }

                    // Collect all nodes in this block for removal.
                    for (var k = start; k <= end; k++) {
                        toRemove.push(insns.get(k));
                    }
                    removed++;
                }

                // Remove collected nodes (by reference, so no index shifting issues)
                for (var r = 0; r < toRemove.length; r++) {
                    insns.remove(toRemove[r]);
                }

                ASMAPI.log('INFO', '[AzaleaBridge] Removed ' + removed + ' problematic config task(s), kept safe tasks');
                return method;
            }
        },

        // --- Patch 3: Skip early registry sync for non-NeoForge clients ---
        'configureEarlyTasks': {
            'target': {
                'type': 'METHOD',
                'class': 'net.neoforged.neoforge.network.ConfigurationInitialization',
                'methodName': 'configureEarlyTasks',
                'methodDesc': '(Lnet/minecraft/network/protocol/configuration/ServerConfigurationPacketListener;Ljava/util/function/Consumer;)V'
            },
            'transformer': function(method) {
                ASMAPI.log('INFO', '[AzaleaBridge] Patching ConfigurationInitialization.configureEarlyTasks()');

                // The original method conditionally adds SyncRegistries().
                // We leave it as-is — the bot doesn't register frozen registry channels
                // so the hasChannel() checks will return false naturally.
                // Nothing to patch here if the negotiate patch works correctly.
                ASMAPI.log('INFO', '[AzaleaBridge] configureEarlyTasks left intact (channel checks will filter)');
                return method;
            }
        },

        // --- Patch 4: Patch entire NetworkRegistry class ---
        // Handles BOTH disconnect() neutralization AND checkPacket ATHROW
        // in a single CLASS-level transformer to avoid conflicts.
        'networkregistry': {
            'target': {
                'type': 'CLASS',
                'name': 'net.neoforged.neoforge.network.registration.NetworkRegistry'
            },
            'transformer': function(classNode) {
                ASMAPI.log('INFO', '[AzaleaBridge] Patching NetworkRegistry (all methods)');

                var methods = classNode.methods;
                var disconnectCount = 0;
                var athrowCount = 0;

                for (var m = 0; m < methods.size(); m++) {
                    var method = methods.get(m);
                    var insns = method.instructions;
                    var methodDisconnects = 0;
                    var methodThrows = 0;

                    for (var i = 0; i < insns.size(); i++) {
                        var insn = insns.get(i);

                        // Neutralize disconnect() calls — replace with POP+POP
                        if (insn instanceof MethodInsnNode && insn.name === 'disconnect') {
                            var pop1 = new InsnNode(Opcodes.POP);
                            var pop2 = new InsnNode(Opcodes.POP);
                            insns.set(insn, pop1);
                            insns.insert(pop1, pop2);
                            methodDisconnects++;
                            i++; // skip the inserted POP
                        }

                        // Suppress ATHROW — replace with POP+RETURN
                        if (insn.getOpcode() === Opcodes.ATHROW) {
                            var popInsn = new InsnNode(Opcodes.POP);
                            insns.set(insn, popInsn);
                            insns.insert(popInsn, new InsnNode(Opcodes.RETURN));
                            methodThrows++;
                            i++; // skip the inserted RETURN
                        }
                    }

                    if (methodDisconnects > 0 || methodThrows > 0) {
                        ASMAPI.log('INFO', '[AzaleaBridge] ' + method.name + '(): ' +
                            methodDisconnects + ' disconnect(s), ' + methodThrows + ' throw(s)');
                    }
                    disconnectCount += methodDisconnects;
                    athrowCount += methodThrows;
                }

                ASMAPI.log('INFO', '[AzaleaBridge] NetworkRegistry total: ' +
                    disconnectCount + ' disconnect(s), ' + athrowCount + ' throw(s) neutralized');
                return classNode;
            }
        }
    };
}
