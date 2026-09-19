#include "cpp/execution_models_cxx.h"
#include "cpp/chart_cxx.h"
#include "q_terminal/src/execution_models.cxxqt.h"

ExecutionModels* make_test_execution_models() {
    ensure_test_app();
    return new ExecutionModels();
}

void delete_test_execution_models(ExecutionModels* models) {
    if (models) {
        delete models;
    }
}

void execution_models_sync(ExecutionModels* models) {
    if (!models) return;
    models->sync();
}

std::int64_t execution_models_revision(ExecutionModels* models) {
    return models ? models->getRevision() : 0;
}

std::int64_t execution_models_redraw_count(ExecutionModels* models) {
    return models ? models->getRedraw_count() : 0;
}

void execution_models_select_deployment(ExecutionModels* models, rust::Str id) {
    if (!models) return;
    models->select_deployment(QString::fromUtf8(id.data(), static_cast<int>(id.size())));
}

void execution_models_select_account(ExecutionModels* models, rust::Str id) {
    if (!models) return;
    models->select_account(QString::fromUtf8(id.data(), static_cast<int>(id.size())));
}

void execution_models_load_older(ExecutionModels* models, rust::Str table) {
    if (!models) return;
    models->load_older(QString::fromUtf8(table.data(), static_cast<int>(table.size())));
}
