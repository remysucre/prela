import java.nio.file.*;
import org.ohdsi.sql.SqlRender;
import org.ohdsi.sql.SqlTranslate;

public class Render {
    public static void main(String[] a) throws Exception {
        String[] names = {"vocabulary_database_schema", "cdm_database_schema", "target_database_schema", "results_database_schema", "target_cohort_table", "target_cohort_id"};
        String[] vals = {"main", "main", "main", "main", "cohort_out", "1"};
        Path out = Paths.get(a[0]);
        for (int i = 1; i < a.length; i++) {
            Path p = Paths.get(a[i]);
            String sql = SqlRender.renderSql(Files.readString(p), names, vals);
            sql = SqlTranslate.translateSql(sql, "duckdb");
            Files.writeString(out.resolve(p.getFileName()), sql);
        }
    }
}
