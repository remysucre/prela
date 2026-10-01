WITH TagStatistics AS (
    SELECT
        t.TagName,
        COUNT(p.Id) AS TotalPosts,
        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
        AVG(u.Reputation) AS AverageReputation
    FROM Tags t
    JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%'
    JOIN Users u ON p.OwnerUserId = u.Id
    GROUP BY t.TagName
),
TopTags AS (
    SELECT
        TagName,
        TotalPosts,
        QuestionsCount,
        AnswersCount,
        AverageReputation,
        RANK() OVER (ORDER BY TotalPosts DESC) AS TagRank
    FROM TagStatistics
),
RecentPosts AS (
    SELECT
        p.Id,
        p.Title,
        p.CreationDate,
        STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) AS Tags
    FROM Posts p
    JOIN Tags t ON p.Tags LIKE '%' || t.TagName || '%'
    WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'
    GROUP BY p.Id, p.Title, p.CreationDate
)
SELECT
    tt.TagName,
    tt.TotalPosts,
    tt.QuestionsCount,
    tt.AnswersCount,
    tt.AverageReputation,
    rp.Title AS RecentPostTitle,
    rp.CreationDate AS RecentPostDate,
    rp.Tags AS RecentPostTags
FROM TopTags tt
LEFT JOIN RecentPosts rp ON rp.Tags LIKE '%' || tt.TagName || '%'
WHERE tt.TagRank <= 10
ORDER BY tt.TagRank, rp.CreationDate DESC;