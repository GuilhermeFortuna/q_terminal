#ifndef Q_TERMINAL_EXECUTION_MODELS_CXX_H
#define Q_TERMINAL_EXECUTION_MODELS_CXX_H

#include <cstdint>
#include "rust/cxx.h"

class ExecutionModels;

ExecutionModels* make_test_execution_models();
void delete_test_execution_models(ExecutionModels* models);
void execution_models_sync(ExecutionModels* models);
std::int64_t execution_models_revision(ExecutionModels* models);
std::int64_t execution_models_redraw_count(ExecutionModels* models);
void execution_models_select_deployment(ExecutionModels* models, rust::Str id);
void execution_models_select_account(ExecutionModels* models, rust::Str id);
void execution_models_load_older(ExecutionModels* models, rust::Str table);

#endif // Q_TERMINAL_EXECUTION_MODELS_CXX_H
